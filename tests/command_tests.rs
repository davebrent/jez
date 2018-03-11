extern crate jez;
extern crate serde_json;

macro_rules! command_test {
    ( $duration:expr, $name:expr ) => (
        // Run a program for a duration and compare its output commands
        let program = include_str!(concat!("files/", $name, ".jez"));
        let expected = include_str!(concat!("files/", $name, ".json"));
        let data = jez::simulate($duration, 0.5, program).unwrap();
        let actual: serde_json::Value = serde_json::from_str(&data).unwrap();
        let expected: serde_json::Value = serde_json::from_str(&expected).unwrap();
        if actual["commands"] != expected {
            println!("{}", serde_json::to_string_pretty(&actual["commands"]).unwrap());
        }
        assert_eq!(actual["commands"], expected);
    );
}

#[test]
fn test_events_simple() {
    command_test!(250.0, "events_simple");
}

#[test]
fn test_last_note_off() {
    command_test!(250.0, "last_note_off");
}

#[test]
fn test_sieves_simple() {
    command_test!(250.0, "sieves_simple");
}

#[test]
fn test_sieves_xor() {
    command_test!(350.0, "sieves_xor");
}

#[test]
fn test_rhythm_onsets() {
    command_test!(300.0, "rhythm_onsets");
}

#[test]
fn test_rotate_simple() {
    command_test!(200.0, "rotate_simple");
}
