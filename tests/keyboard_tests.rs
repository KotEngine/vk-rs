use vkontakte::tools::keyboard::{ButtonAction, ButtonColor, Keyboard};

#[test]
fn keyboard_builder_json() {
    let mut kb = Keyboard::new(false, false);
    kb.row()
        .add(ButtonAction::text("Button 1"), Some(ButtonColor::Primary))
        .add(ButtonAction::text("Button 2"), Some(ButtonColor::Secondary));

    let json = kb.to_json();
    assert!(json.contains("Button 1"));
    assert!(json.contains("primary"));
    assert!(json.contains("secondary"));
}

#[test]
fn inline_keyboard_flag() {
    let kb = Keyboard::new(false, true);
    let json = kb.to_json();
    assert!(json.contains("\"inline\":true"));
}

#[test]
fn callback_button_payload() {
    let mut kb = Keyboard::new(true, true);
    kb.row()
        .add(ButtonAction::callback("Click", "payload123"), Some(ButtonColor::Positive));

    let json = kb.to_json();
    assert!(json.contains("callback"));
    assert!(json.contains("payload123"));
}
