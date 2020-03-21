use serde_json::json;

use crate::telegram::types::Message;

const SEND_MESSAGE_SUCCESS: &str = r#"{"ok":true,"result":{"message_id":691,"from":{"id":414141,"is_bot":true,"first_name":"Bot","username":"Bot"},"chat":{"id":123,"first_name":"Name","username":"username","type":"private"},"date":1581200384,"text":"This is a test message"}}"#;

#[cfg(test)]
pub mod test_helpers {
    use super::*;
    use mockito::{mock, Matcher, Mock};

    pub fn mock_send_message_called(token: &str, message: &Message) -> Mock {
        mock("POST", format!("/bot{}/sendMessage", token).as_str())
            .match_body(Matcher::Json(json!(message)))
            .with_status(200)
            .with_body(SEND_MESSAGE_SUCCESS)
            .with_header("content-type", "application/json")
            .expect(1)
            .create()
    }

    pub fn mock_send_message_not_called(token: &str) -> Mock {
        mock("POST", format!("/bot{}/sendMessage", token).as_str())
            .expect(0)
            .create()
    }
}
