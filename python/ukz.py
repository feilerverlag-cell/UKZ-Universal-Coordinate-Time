"""UKZ v0.5.0 draft: integer core and explicit UTC/TAI adapters."""
from dataclasses import dataclass
from datetime import datetime, timezone, timedelta
from enum import Enum
import json, re
from pathlib import Path

TICK_MICROS=864_000
TICKS_PER_DAY=100_000
TICKS_PER_YEAR=52_000_000
UNIX_EPOCH=datetime(1970,1,1,tzinfo=timezone.utc)
EPOCH_POSIX_MICROS=1_704_067_200_000_000
EPOCH_TAI_MICROS=1_704_067_237_000_000

@dataclass(frozen=True, order=True)
class UKZ:
    ticks:int
    def __post_init__(self):
        if type(self.ticks) is not int: raise TypeError("ticks must be int")
        if not -(2**63)<=self.ticks<=2**63-1: raise OverflowError("ticks outside i64")
    @property
    def second(self): return self.ticks%100
    @property
    def minute(self): return (self.ticks%10_000)//100
    @property
    def hour(self): return (self.ticks%100_000)//10_000
    @property
    def day_of_ukz_week(self): return (self.ticks//TICKS_PER_DAY)%10
    @property
    def week_of_ukz_year(self): return ((self.ticks//TICKS_PER_DAY)%520)//10
    @property
    def day_of_ukz_year(self): return (self.ticks//TICKS_PER_DAY)%520
    @property
    def year(self): return self.ticks//TICKS_PER_YEAR

class ConversionStatus(Enum):
    CONFIRMED="Confirmed"
    PROVISIONAL="Provisional"
    UNKNOWN="Unknown"

class UtcError(ValueError): pass

@dataclass(frozen=True)
class LeapEntry:
    effective_utc:datetime
    tai_minus_utc:int

class LeapSecondTable:
    def __init__(self, entries, confirmed_through):
        self.entries=tuple(entries)
        self.confirmed_through=confirmed_through
        if not self.entries or self.entries[0].effective_utc!=datetime(1972,1,1,tzinfo=timezone.utc):
            raise UtcError("table must begin at 1972-01-01 UTC")
        previous=None
        for e in self.entries:
            if e.effective_utc.tzinfo is None or e.effective_utc.utcoffset()!=timedelta(0):
                raise UtcError("entry timestamps must be UTC")
            if previous:
                if e.effective_utc<=previous[0]: raise UtcError("entries must be strictly ordered")
                if abs(e.tai_minus_utc-previous[1])!=1: raise UtcError("offset steps must be one second")
            previous=(e.effective_utc,e.tai_minus_utc)
    @classmethod
    def load(cls,path):
        raw=json.loads(Path(path).read_text(encoding="utf-8"))
        es=[LeapEntry(_parse_normal(x["effective_utc"]),int(x["tai_minus_utc"])) for x in raw["entries"]]
        return cls(es,_parse_normal(raw["confirmed_through"]))
    def offset_at(self,dt):
        if dt<self.entries[0].effective_utc: raise UtcError("UTC before 1972 unsupported")
        offset=self.entries[0].tai_minus_utc
        for e in self.entries[1:]:
            if dt<e.effective_utc: break
            offset=e.tai_minus_utc
        return offset, (ConversionStatus.CONFIRMED if dt<=self.confirmed_through else ConversionStatus.PROVISIONAL)
    def transition_after_day(self,day):
        midnight=day+timedelta(days=1)
        for i in range(1,len(self.entries)):
            e=self.entries[i]
            if e.effective_utc==midnight:
                return self.entries[i-1].tai_minus_utc,e.tai_minus_utc
        return None

class TaiAdapter:
    @staticmethod
    def from_tai_micros(value):
        if type(value) is not int: raise TypeError("TAI micros must be int")
        return UKZ((value-EPOCH_TAI_MICROS)//TICK_MICROS)

class PosixAdapter:
    @staticmethod
    def to_ukz(dt):
        if dt.tzinfo is None or dt.utcoffset() is None: raise UtcError("timezone required")
        u=dt.astimezone(timezone.utc)
        micros=(u-UNIX_EPOCH)//timedelta(microseconds=1)
        return UKZ((micros-EPOCH_POSIX_MICROS)//TICK_MICROS)

_RX=re.compile(r"^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})(?:\.(\d{1,6}))?Z$")
def _parts(s):
    m=_RX.fullmatch(s)
    if not m: raise UtcError("expected YYYY-MM-DDTHH:MM:SS[.ffffff]Z")
    y,mo,d,h,mi,se=map(int,m.groups()[:6])
    us=int((m.group(7) or "").ljust(6,"0") or "0")
    try: day=datetime(y,mo,d,tzinfo=timezone.utc)
    except ValueError as ex: raise UtcError(str(ex)) from ex
    if h>23 or mi>59 or se>60: raise UtcError("time component out of range")
    return day,h,mi,se,us
def _parse_normal(s):
    day,h,mi,se,us=_parts(s)
    if se==60: raise UtcError("second 60 requires leap-second path")
    return day.replace(hour=h,minute=mi,second=se,microsecond=us)

class UtcAdapter:
    @staticmethod
    def from_utc_string(s,table):
        day,h,mi,se,us=_parts(s)
        if se==60:
            if (h,mi)!=(23,59): raise UtcError("23:59:60 is only valid at day end")
            change=table.transition_after_day(day)
            if change is None: raise UtcError("no leap second on this date")
            old,new=change
            if new!=old+1: raise UtcError("this transition is not a positive leap second")
            next_midnight=day+timedelta(days=1)
            posix_us=(next_midnight-UNIX_EPOCH)//timedelta(microseconds=1)+us
            status=ConversionStatus.CONFIRMED if next_midnight<=table.confirmed_through+timedelta(microseconds=1) else ConversionStatus.PROVISIONAL
            return TaiAdapter.from_tai_micros(posix_us+old*1_000_000),status
        dt=day.replace(hour=h,minute=mi,second=se,microsecond=us)
        offset,status=table.offset_at(dt)
        posix_us=(dt-UNIX_EPOCH)//timedelta(microseconds=1)
        return TaiAdapter.from_tai_micros(posix_us+offset*1_000_000),status
