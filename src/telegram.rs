use std::error::Error;

use reqwest::{Client, Response};
use serde::Serialize;
use serde_json::{from_str, Value};

pub struct TelegramClient {
    token: String,
    chat_id: String,
    domain: String,
}

#[derive(Serialize)]
struct Message<'a> {
    chat_id: &'a str,
    text: &'a str,
    disable_notification: bool,
    disable_web_page_preview: bool,
}

#[derive(Serialize)]
struct Image<'a> {
    chat_id: &'a str,
    photo: &'a str,
    disable_notification: bool,
}

impl TelegramClient {
    pub fn new(token: String, chat_id: String) -> TelegramClient {
        TelegramClient {
            token,
            chat_id,
            domain: String::from("https://api.telegram.org"),
        }
    }

    #[allow(dead_code)]
    pub fn new_with(token: String, chat_id: String, domain: String) -> TelegramClient {
        TelegramClient {
            token,
            chat_id,
            domain,
        }
    }

    pub async fn send_message(&self, text: &str) -> Result<String, Box<dyn Error>> {
        let message = Message {
            chat_id: &self.chat_id,
            text,
            disable_notification: true,
            disable_web_page_preview: true,
        };

        let url = format!("{}/bot{}/sendMessage", self.domain, self.token);
        let resp: Response = Client::new().post(&url).json(&message).send().await?;

        if resp.status().is_success() {
            let resp: Value = from_str(&resp.text().await?)?;
            let resp = &resp["result"];
            let resp = &resp["message_id"];
            Ok(format!("{}", resp))
        } else {
            Err(resp.text().await?.into())
        }
    }

    pub async fn send_image(&self, image_url: &str) -> Result<String, Box<dyn Error>> {
        let image = Image {
            chat_id: &self.chat_id,
            photo: image_url,
            disable_notification: true,
        };
        let url = format!("{}/bot{}/sendPhoto", self.domain, self.token);
        let resp: Response = Client::new().post(&url).json(&image).send().await?;

        if resp.status().is_success() {
            let resp: Value = from_str(&resp.text().await?)?;
            let resp = &resp["result"];
            let resp = &resp["message_id"];
            Ok(format!("{}", resp))
        } else {
            Err(resp.text().await?.into())
        }
    }

    pub async fn delete_message(&self, message_id: &str) -> Result<(), Box<dyn Error>> {
        let url = format!("{}/bot{}/deleteMessage", self.domain, self.token);
        let resp: Response = Client::new()
            .post(&url)
            .form(&[
                ("chat_id", &self.chat_id),
                ("message_id", &String::from(message_id)),
            ])
            .send()
            .await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(resp.text().await?.into())
        }
    }

    pub async fn edit_message_text(
        &self,
        message_id: &str,
        text: &str,
    ) -> Result<(), Box<dyn Error>> {
        let url = format!("{}/bot{}/editMessageText", self.domain, self.token);
        let resp: Response = Client::new()
            .post(&url)
            .form(&[
                ("chat_id", &self.chat_id),
                ("message_id", &String::from(message_id)),
                ("text", &String::from(text)),
            ])
            .send()
            .await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(resp.text().await?.into())
        }
    }

    pub async fn edit_message_image(
        &self,
        message_id: &str,
        image_url: &str,
    ) -> Result<(), Box<dyn Error>> {
        let url = format!("{}/bot{}/editMessageMedia", self.domain, self.token);
        let body: serde_json::Value = serde_json::from_str(
            format!(
                r#"
        {{
            "chat_id": "{}",
            "message_id": "{}",
            "media": {{
                "type": "photo",
                "media": "{}"
            }}
        }}
        "#,
                &self.chat_id, message_id, image_url
            )
            .as_str(),
        )
        .unwrap();
        let resp: Response = Client::new().post(&url).json(&body).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(resp.text().await?.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use mockito::{mock, server_url, Matcher};
    use serde_json::json;

    use super::*;

    const TOKEN: &str = "token";
    const CHAT_ID: &str = "123";

    #[test]
    fn correct_domain() {
        let telegram_client = TelegramClient::new(String::from(TOKEN), String::from(CHAT_ID));
        assert_eq!(telegram_client.domain, "https://api.telegram.org");
    }

    #[tokio::test]
    async fn send_message_success() {
        let url = &server_url();
        let resp = r#"{"ok":true,"result":{"message_id":691,"from":{"id":414141,"is_bot":true,"first_name":"KliversalaBot","username":"KliversalaBot"},"chat":{"id":123,"first_name":"Name","username":"username","type":"private"},"date":1581200384,"text":"This is a test message"}}"#;

        let text = "message text";
        let expected_message = Message {
            chat_id: CHAT_ID,
            text,
            disable_notification: true,
            disable_web_page_preview: true,
        };

        let _m = mock("POST", format!("/bot{}/sendMessage", TOKEN).as_str())
            .match_body(Matcher::Json(json!(expected_message)))
            .with_status(200)
            .with_body(resp)
            .with_header("content-type", "application/json")
            .create();

        let client = TelegramClient::new_with(
            String::from(TOKEN),
            String::from(CHAT_ID),
            String::from(url),
        );

        let result = client.send_message(text).await.unwrap();
        assert_eq!(result, "691");
        _m.assert();
    }

    #[tokio::test]
    async fn send_message_error() {
        let error = r#"{"ok":false,"error_code":400,"description":"Bad Request: chat not found"}"#;
        let url = &server_url();

        let text = "message text";
        let expected_message = Message {
            chat_id: CHAT_ID,
            text,
            disable_notification: true,
            disable_web_page_preview: true,
        };

        let _m = mock("POST", format!("/bot{}/sendMessage", TOKEN).as_str())
            .match_body(Matcher::Json(json!(expected_message)))
            .with_status(400)
            .with_header("content-type", "application/json")
            .with_body(error)
            .create();

        let client = TelegramClient::new_with(
            String::from(TOKEN),
            String::from(CHAT_ID),
            String::from(url),
        );

        let result = client.send_message(text).await.unwrap_err();
        let result = format!("{}", result);
        assert_eq!(result, error);
        _m.assert();
    }

    #[tokio::test]
    async fn send_image_success() {
        let url = &server_url();
        let resp = r#"{"ok":true,"result":{"message_id":691,"from":{"id":414141,"is_bot":true,"first_name":"KliversalaBot","username":"KliversalaBot"},"chat":{"id":123,"first_name":"Name","username":"username","type":"private"},"date":1581200384,"text":"This is a test message"}}"#;

        let image_url = "image url";
        let expected_image = Image {
            chat_id: CHAT_ID,
            photo: image_url,
            disable_notification: true,
        };

        let _m = mock("POST", format!("/bot{}/sendPhoto", TOKEN).as_str())
            .match_body(Matcher::Json(json!(expected_image)))
            .with_status(200)
            .with_body(resp)
            .with_header("content-type", "application/json")
            .create();

        let client = TelegramClient::new_with(
            String::from(TOKEN),
            String::from(CHAT_ID),
            String::from(url),
        );

        let result = client.send_image(image_url).await.unwrap();
        let result = format!("{}", result);
        assert_eq!(result, "691");
        _m.assert();
    }

    #[tokio::test]
    async fn send_image_error() {
        let error = r#"{"ok":false,"error_code":400,"description":"Bad Request: chat not found"}"#;
        let url = &server_url();

        let image_url = "image url";
        let expected_image = Image {
            chat_id: CHAT_ID,
            photo: image_url,
            disable_notification: true,
        };

        let _m = mock("POST", format!("/bot{}/sendPhoto", TOKEN).as_str())
            .match_body(Matcher::Json(json!(expected_image)))
            .with_status(400)
            .with_header("content-type", "application/json")
            .with_body(error)
            .create();

        let client = TelegramClient::new_with(
            String::from(TOKEN),
            String::from(CHAT_ID),
            String::from(url),
        );

        let result = client.send_image(image_url).await.unwrap_err();
        let result = format!("{}", result);
        assert_eq!(result, error);
        _m.assert();
    }

    #[tokio::test]
    async fn delete_message_success() {
        let url = &server_url();
        let _m = mock("POST", format!("/bot{}/deleteMessage", TOKEN).as_str())
            .with_status(200)
            .with_header("content-type", "application/json")
            .create();

        let client = TelegramClient::new_with(
            String::from(TOKEN),
            String::from(CHAT_ID),
            String::from(url),
        );

        let result = client.delete_message("id").await.unwrap();
        assert_eq!(result, ());
        _m.assert();
    }

    #[tokio::test]
    async fn delete_message_error() {
        let error = r#"{"ok":false,"error_code":400,"description":"Bad Request: chat not found"}"#;

        let url = &server_url();
        let _m = mock("POST", format!("/bot{}/deleteMessage", TOKEN).as_str())
            .with_status(400)
            .with_body(error)
            .with_header("content-type", "application/json")
            .create();

        let client = TelegramClient::new_with(
            String::from(TOKEN),
            String::from(CHAT_ID),
            String::from(url),
        );

        let result = client.delete_message("id").await.unwrap_err();
        let result = format!("{}", result);
        assert_eq!(result, error);
        _m.assert();
    }

    #[tokio::test]
    async fn edit_message_text_success() {
        let url = &server_url();
        let text = "text";
        let message_id = "id";

        let _m = mock("POST", format!("/bot{}/editMessageText", TOKEN).as_str())
            .match_body(Matcher::AllOf(vec![
                Matcher::UrlEncoded(String::from("chat_id"), String::from(CHAT_ID)),
                Matcher::UrlEncoded(String::from("message_id"), String::from(message_id)),
                Matcher::UrlEncoded(String::from("text"), String::from(text)),
            ]))
            .with_status(200)
            .with_body("success")
            .with_header("content-type", "application/json")
            .create();

        let client = TelegramClient::new_with(
            String::from(TOKEN),
            String::from(CHAT_ID),
            String::from(url),
        );

        let result = client.edit_message_text(message_id, text).await.unwrap();
        assert_eq!(result, ());
        _m.assert();
    }

    #[tokio::test]
    async fn edit_message_text_error() {
        let error = r#"{"ok":false,"error_code":400,"description":"Bad Request: chat not found"}"#;
        let url = &server_url();
        let text = "text";
        let message_id = "id";

        let _m = mock("POST", format!("/bot{}/editMessageText", TOKEN).as_str())
            .match_body(Matcher::AnyOf(vec![
                Matcher::UrlEncoded(String::from("chat_id"), String::from(CHAT_ID)),
                Matcher::UrlEncoded(String::from("message_id"), String::from(message_id)),
                Matcher::UrlEncoded(String::from("text"), String::from(text)),
            ]))
            .with_status(400)
            .with_header("content-type", "application/json")
            .with_body(error)
            .create();

        let client = TelegramClient::new_with(
            String::from(TOKEN),
            String::from(CHAT_ID),
            String::from(url),
        );

        let result = client
            .edit_message_text(message_id, text)
            .await
            .unwrap_err();
        let result = format!("{}", result);
        assert_eq!(result, error);
        _m.assert();
    }

    #[tokio::test]
    async fn edit_message_image_success() {
        let url = &server_url();
        let image_url = "image-url";
        let message_id = "id";

        let body: serde_json::Value = serde_json::from_str(
            format!(
                r#"
                    {{
                        "chat_id": "{}",
                        "message_id": "{}",
                        "media": {{
                            "type": "photo",
                            "media": "{}"
                        }}
                    }}
                "#,
                CHAT_ID, message_id, image_url
            )
            .as_str(),
        )
        .unwrap();

        let _m = mock("POST", format!("/bot{}/editMessageMedia", TOKEN).as_str())
            .match_body(Matcher::Json(body))
            .with_status(200)
            .with_body("success")
            .with_header("content-type", "application/json")
            .create();

        let client = TelegramClient::new_with(
            String::from(TOKEN),
            String::from(CHAT_ID),
            String::from(url),
        );

        let result = client
            .edit_message_image(message_id, image_url)
            .await
            .unwrap();
        assert_eq!(result, ());
        _m.assert();
    }

    #[tokio::test]
    async fn edit_message_image_error() {
        let error = r#"{"ok":false,"error_code":400,"description":"Bad Request: chat not found"}"#;
        let url = &server_url();

        let image_url = "image-url";
        let message_id = "id";

        let body: serde_json::Value = serde_json::from_str(
            format!(
                r#"
                    {{
                        "chat_id": "{}",
                        "message_id": "{}",
                        "media": {{
                            "type": "photo",
                            "media": "{}"
                        }}
                    }}
                "#,
                CHAT_ID, message_id, image_url
            )
            .as_str(),
        )
        .unwrap();

        let _m = mock("POST", format!("/bot{}/editMessageMedia", TOKEN).as_str())
            .match_body(Matcher::Json(body))
            .with_status(400)
            .with_header("content-type", "application/json")
            .with_body(error)
            .create();

        let client = TelegramClient::new_with(
            String::from(TOKEN),
            String::from(CHAT_ID),
            String::from(url),
        );

        let result = client
            .edit_message_image(message_id, image_url)
            .await
            .unwrap_err();
        let result = format!("{}", result);
        assert_eq!(result, error);
        _m.assert();
    }
}
