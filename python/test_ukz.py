import json
import unittest
from pathlib import Path
from datetime import datetime, timezone, timedelta

from ukz import (
    UKZ,
    PosixAdapter,
    UtcAdapter,
    TaiAdapter,
    LeapSecondTable,
    ConversionStatus,
)

ROOT = Path(__file__).resolve().parents[1]


class TestUKZ(unittest.TestCase):

    @classmethod
    def setUpClass(cls):
        cls.table = LeapSecondTable.load(
            ROOT / "ukz_leap_seconds.json"
        )

    def test_shared_posix_vectors(self):
        data = json.loads(
            (ROOT / "ukz_conformance_vectors.json").read_text()
        )

        self.assertEqual(data["specification_version"], "1.0")
        self.assertEqual(data["input_scale"], "POSIX")

        epoch = datetime(1970, 1, 1, tzinfo=timezone.utc)

        for v in data["test_vectors"]:
            dt = epoch + timedelta(
                microseconds=v["unix_micros"]
            )

            u = PosixAdapter.to_ukz(dt)

            self.assertEqual(
                u.ticks,
                v["expected_ticks"]
            )

            c = v["expected_calendar"]

            self.assertEqual(
                (
                    u.year,
                    u.day_of_ukz_year,
                    u.hour,
                    u.minute,
                    u.second,
                ),
                (
                    c["year"],
                    c["day"],
                    c["hour"],
                    c["minute"],
                    c["second"],
                ),
            )

    def test_2024_epoch_utc_and_tai_agree(self):
        u, status = UtcAdapter.from_utc_string(
            "2024-01-01T00:00:00Z",
            self.table,
        )

        self.assertEqual(u, UKZ(0))
        self.assertEqual(status, ConversionStatus.CONFIRMED)

        self.assertEqual(
            TaiAdapter.from_tai_micros(
                1_704_067_237_000_000
            ),
            UKZ(0),
        )

    def test_2016_leap_second_is_distinct_and_contiguous(self):
        a, _ = UtcAdapter.from_utc_string(
            "2016-12-31T23:59:59Z",
            self.table,
        )

        b, _ = UtcAdapter.from_utc_string(
            "2016-12-31T23:59:60Z",
            self.table,
        )

        c, _ = UtcAdapter.from_utc_string(
            "2017-01-01T00:00:00Z",
            self.table,
        )

        self.assertLess(a.ticks, b.ticks)
        self.assertLess(b.ticks, c.ticks)

    def test_invalid_leap_second_rejected(self):
        with self.assertRaises(ValueError):
            UtcAdapter.from_utc_string(
                "2016-12-30T23:59:60Z",
                self.table,
            )

    def test_pre_1972_unknown_rejected(self):
        with self.assertRaises(ValueError):
            UtcAdapter.from_utc_string(
                "1971-12-31T23:59:59Z",
                self.table,
            )


if __name__ == "__main__":
    unittest.main()
