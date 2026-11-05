use ukz_adapters::{
    ConversionStatus, LeapSecondTable, TaiAdapter, Ukz, UtcAdapter,
};

#[test]
fn epoch_utc_and_tai_agree() {
    let table = LeapSecondTable::from_json_file(
        "../../ukz_leap_seconds.json"
    ).unwrap();

    let (utc, status) =
        UtcAdapter::from_utc_string("2024-01-01T00:00:00Z", &table)
            .unwrap();

    let tai = TaiAdapter::from_tai_micros(1_704_067_237_000_000)
        .unwrap();

    assert_eq!(utc, Ukz::new(0));
    assert_eq!(tai, Ukz::new(0));
    assert_eq!(status, ConversionStatus::Confirmed);
}

#[test]
fn leap_second_is_recognized() {
    let table = LeapSecondTable::from_json_file(
        "../../ukz_leap_seconds.json"
    ).unwrap();

    let before =
        UtcAdapter::from_utc_string("2016-12-31T23:59:59Z", &table)
            .unwrap().0;

    let leap =
        UtcAdapter::from_utc_string("2016-12-31T23:59:60Z", &table)
            .unwrap().0;

    let after =
        UtcAdapter::from_utc_string("2017-01-01T00:00:00Z", &table)
            .unwrap().0;

    assert!(before.ticks() < leap.ticks());
    assert!(leap.ticks() < after.ticks());
}

#[test]
fn invalid_leap_second_is_rejected() {
    let table = LeapSecondTable::from_json_file(
        "../../ukz_leap_seconds.json"
    ).unwrap();

    let result =
        UtcAdapter::from_utc_string("2016-12-30T23:59:60Z", &table);

    assert!(result.is_err());
}
