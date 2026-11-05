#[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord)]
pub struct Ukz { ticks:i64 }
impl Ukz {
 pub const TICKS_PER_DAY:i64=100_000;
 pub const TICKS_PER_YEAR:i64=52_000_000;
 pub const TICK_MICROS:i64=864_000;
 pub const fn new(ticks:i64)->Self{Self{ticks}}
 pub const fn ticks(self)->i64{self.ticks}
 pub fn checked_add(self,d:i64)->Option<Self>{self.ticks.checked_add(d).map(Self::new)}
 pub fn checked_sub(self,rhs:Self)->Option<i64>{self.ticks.checked_sub(rhs.ticks)}
 pub const fn second(self)->i64{self.ticks.rem_euclid(100)}
 pub const fn minute(self)->i64{self.ticks.rem_euclid(10_000)/100}
 pub const fn hour(self)->i64{self.ticks.rem_euclid(100_000)/10_000}
 pub const fn day_of_ukz_week(self)->i64{self.ticks.div_euclid(Self::TICKS_PER_DAY).rem_euclid(10)}
 pub const fn week_of_ukz_year(self)->i64{self.ticks.div_euclid(Self::TICKS_PER_DAY).rem_euclid(520)/10}
 pub const fn day_of_ukz_year(self)->i64{self.ticks.div_euclid(Self::TICKS_PER_DAY).rem_euclid(520)}
 pub const fn year(self)->i64{self.ticks.div_euclid(Self::TICKS_PER_YEAR)}
}
