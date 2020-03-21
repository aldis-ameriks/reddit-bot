use std::error::Error;

use reqwest::{Client, Response};
use serde_json::{from_str, Value};

use super::error::TelegramError;
use super::types::*;

pub struct TelegramClient {
    token: String,
    domain: String,
}

impl TelegramClient {
    pub fn new(token: String) -> TelegramClient {
        TelegramClient {
            token,
            domain: String::from("https://api.telegram.org"),
        }
    }

    #[allow(dead_code)]
    pub fn new_with(token: String, domain: String) -> TelegramClient {
        TelegramClient { token, domain }
    }

    pub async fn send_message<'a>(&self, message: &Message<'a>) -> Result<String, TelegramError> {
        let url = format!("{}/bot{}/sendMessage", self.domain, self.token);
        let resp: Response = Client::new().post(&url).json(message).send().await?;

        if resp.status().is_success() {
            let resp: Value = from_str(&resp.text().await?)?;
            let resp = &resp["result"];
            let resp = &resp["message_id"];
            Ok(format!("{}", resp))
        } else {
            Err(resp.text().await?.into())
        }
    }

    #[allow(dead_code)]
    pub async fn send_photo<'a>(&self, image: &Image<'a>) -> Result<String, TelegramError> {
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

    #[allow(dead_code)]
    pub async fn delete_message(
        &self,
        chat_id: &str,
        message_id: &str,
    ) -> Result<(), TelegramError> {
        let url = format!("{}/bot{}/deleteMessage", self.domain, self.token);
        let resp: Response = Client::new()
            .post(&url)
            .form(&[
                ("chat_id", &String::from(chat_id)),
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

    #[allow(dead_code)]
    pub async fn edit_message_text<'a>(
        &self,
        message: &EditMessage<'a>,
    ) -> Result<(), Box<dyn Error>> {
        let url = format!("{}/bot{}/editMessageText", self.domain, self.token);
        let resp: Response = Client::new().post(&url).json(&message).send().await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(resp.text().await?.into())
        }
    }

    #[allow(dead_code)]
    pub async fn edit_message_image<'a>(
        &self,
        edit_image: &EditImage<'a>,
    ) -> Result<(), Box<dyn Error>> {
        let url = format!("{}/bot{}/editMessageMedia", self.domain, self.token);
        let resp: Response = Client::new().post(&url).json(&edit_image).send().await?;

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
    use crate::telegram::test_helpers::mock_send_message_success;

    const TOKEN: &str = "token";

    #[test]
    fn correct_domain() {
        let telegram_client = TelegramClient::new(String::from(TOKEN));
        assert_eq!(telegram_client.domain, "https://api.telegram.org");
    }

    #[tokio::test]
    async fn send_message_success() {
        let url = &server_url();
        let inline_keyboard = vec![vec![InlineKeyboardButton {
            text: "test".to_string(),
            callback_data: "callback_data".to_string(),
        }]];
        let markup = InlineKeyboardMarkup { inline_keyboard };
        let reply_markup = ReplyMarkup::InlineKeyboardMarkup(markup);
        let text = "message text";
        let message = Message {
            chat_id: "123",
            text,
            disable_notification: true,
            disable_web_page_preview: false,
            reply_markup: Some(&reply_markup),
        };
        let _m = mock_send_message_success(TOKEN, &message);
        let client = TelegramClient::new_with(String::from(TOKEN), String::from(url));

        let result = client.send_message(&message).await.unwrap();
        assert_eq!(result, "691");
        _m.assert();
    }

    #[tokio::test]
    async fn send_message_error() {
        let url = &server_url();
        let error = r#"{"ok":false,"error_code":400,"description":"Bad Request: chat not found"}"#;
        let text = "message text";
        let message = Message {
            chat_id: "123",
            text,
            disable_notification: true,
            disable_web_page_preview: false,
            reply_markup: None,
        };

        let _m = mock("POST", format!("/bot{}/sendMessage", TOKEN).as_str())
            .match_body(Matcher::Json(json!(message)))
            .with_status(400)
            .with_header("content-type", "application/json")
            .with_body(error)
            .create();

        let client = TelegramClient::new_with(String::from(TOKEN), String::from(url));
        let result = client.send_message(&message).await.unwrap_err();
        let result = format!("{}", result);
        assert_eq!(result, error);
        _m.assert();
    }

    #[tokio::test]
    async fn send_image_success() {
        let url = &server_url();
        let resp = r#"{"ok":true,"result":{"message_id":691,"from":{"id":414141,"is_bot":true,"first_name":"Bot","username":"Bot"},"chat":{"id":123,"first_name":"Name","username":"username","type":"private"},"date":1581200384,"text":"This is a test message"}}"#;
        let image = Image {
            chat_id: "123",
            photo: "image url",
            disable_notification: true,
        };

        let _m = mock("POST", format!("/bot{}/sendPhoto", TOKEN).as_str())
            .match_body(Matcher::Json(json!(image)))
            .with_status(200)
            .with_body(resp)
            .with_header("content-type", "application/json")
            .create();

        let client = TelegramClient::new_with(String::from(TOKEN), String::from(url));
        let result = client.send_photo(&image).await.unwrap();
        let result = format!("{}", result);
        assert_eq!(result, "691");
        _m.assert();
    }

    #[tokio::test]
    async fn send_image_error() {
        let url = &server_url();
        let error = r#"{"ok":false,"error_code":400,"description":"Bad Request: chat not found"}"#;
        let image = Image {
            chat_id: "123",
            photo: "image url",
            disable_notification: true,
        };

        let _m = mock("POST", format!("/bot{}/sendPhoto", TOKEN).as_str())
            .match_body(Matcher::Json(json!(image)))
            .with_status(400)
            .with_header("content-type", "application/json")
            .with_body(error)
            .create();

        let client = TelegramClient::new_with(String::from(TOKEN), String::from(url));
        let result = client.send_photo(&image).await.unwrap_err();
        let result = format!("{}", result);
        assert_eq!(result, error);
        _m.assert();
    }

    #[tokio::test]
    async fn delete_message_success() {
        let url = &server_url();
        let chat_id = "123";
        let message_id = "456";

        let _m = mock("POST", format!("/bot{}/deleteMessage", TOKEN).as_str())
            .match_body(Matcher::AllOf(vec![
                Matcher::UrlEncoded(String::from("chat_id"), String::from(chat_id)),
                Matcher::UrlEncoded(String::from("message_id"), String::from(message_id)),
            ]))
            .with_status(200)
            .with_header("content-type", "application/json")
            .create();

        let client = TelegramClient::new_with(String::from(TOKEN), String::from(url));
        let result = client.delete_message(chat_id, message_id).await.unwrap();
        assert_eq!(result, ());
        _m.assert();
    }

    #[tokio::test]
    async fn delete_message_error() {
        let url = &server_url();
        let chat_id = "123";
        let message_id = "456";
        let error = r#"{"ok":false,"error_code":400,"description":"Bad Request: chat not found"}"#;
        let _m = mock("POST", format!("/bot{}/deleteMessage", TOKEN).as_str())
            .match_body(Matcher::AllOf(vec![
                Matcher::UrlEncoded(String::from("chat_id"), String::from(chat_id)),
                Matcher::UrlEncoded(String::from("message_id"), String::from(message_id)),
            ]))
            .with_status(400)
            .with_body(error)
            .with_header("content-type", "application/json")
            .create();

        let client = TelegramClient::new_with(String::from(TOKEN), String::from(url));
        let result = client
            .delete_message(chat_id, message_id)
            .await
            .unwrap_err();
        let result = format!("{}", result);
        assert_eq!(result, error);
        _m.assert();
    }

    #[tokio::test]
    async fn edit_message_text_success() {
        let url = &server_url();
        let inline_keyboard = vec![vec![InlineKeyboardButton {
            text: "test".to_string(),
            callback_data: "callback_data".to_string(),
        }]];
        let markup = InlineKeyboardMarkup { inline_keyboard };
        let reply_markup = ReplyMarkup::InlineKeyboardMarkup(markup);
        let text = "message text";
        let message = EditMessage {
            chat_id: "123",
            message_id: "456",
            text,
            disable_notification: true,
            disable_web_page_preview: false,
            reply_markup: Some(&reply_markup),
        };

        let _m = mock("POST", format!("/bot{}/editMessageText", TOKEN).as_str())
            .match_body(Matcher::Json(json!(message)))
            .with_status(200)
            .with_body("success")
            .with_header("content-type", "application/json")
            .create();

        let client = TelegramClient::new_with(String::from(TOKEN), String::from(url));
        let result = client.edit_message_text(&message).await.unwrap();
        assert_eq!(result, ());
        _m.assert();
    }

    #[tokio::test]
    async fn edit_message_text_error() {
        let url = &server_url();
        let error = r#"{"ok":false,"error_code":400,"description":"Bad Request: chat not found"}"#;
        let inline_keyboard = vec![vec![InlineKeyboardButton {
            text: "test".to_string(),
            callback_data: "callback_data".to_string(),
        }]];
        let markup = InlineKeyboardMarkup { inline_keyboard };
        let reply_markup = ReplyMarkup::InlineKeyboardMarkup(markup);
        let text = "message text";
        let message = EditMessage {
            chat_id: "123",
            message_id: "456",
            text,
            disable_notification: true,
            disable_web_page_preview: false,
            reply_markup: Some(&reply_markup),
        };

        let _m = mock("POST", format!("/bot{}/editMessageText", TOKEN).as_str())
            .match_body(Matcher::Json(json!(message)))
            .with_status(400)
            .with_header("content-type", "application/json")
            .with_body(error)
            .create();

        let client = TelegramClient::new_with(String::from(TOKEN), String::from(url));
        let result = client.edit_message_text(&message).await.unwrap_err();
        let result = format!("{}", result);
        assert_eq!(result, error);
        _m.assert();
    }

    #[tokio::test]
    async fn edit_message_image_success() {
        let url = &server_url();
        let media = Media { type_: "photo" };
        let edit_image = EditImage {
            chat_id: "123",
            message_id: "456",
            photo: "image url",
            disable_notification: true,
            media,
        };

        let _m = mock("POST", format!("/bot{}/editMessageMedia", TOKEN).as_str())
            .match_body(Matcher::Json(json!(edit_image)))
            .with_status(200)
            .with_body("success")
            .with_header("content-type", "application/json")
            .create();

        let client = TelegramClient::new_with(String::from(TOKEN), String::from(url));
        let result = client.edit_message_image(&edit_image).await.unwrap();
        assert_eq!(result, ());
        _m.assert();
    }

    #[tokio::test]
    async fn edit_message_image_error() {
        let url = &server_url();
        let error = r#"{"ok":false,"error_code":400,"description":"Bad Request: chat not found"}"#;
        let media = Media { type_: "photo" };
        let edit_image = EditImage {
            chat_id: "123",
            message_id: "456",
            photo: "image url",
            disable_notification: true,
            media,
        };

        let _m = mock("POST", format!("/bot{}/editMessageMedia", TOKEN).as_str())
            .match_body(Matcher::Json(json!(edit_image)))
            .with_status(400)
            .with_header("content-type", "application/json")
            .with_body(error)
            .create();

        let client = TelegramClient::new_with(String::from(TOKEN), String::from(url));
        let result = client.edit_message_image(&edit_image).await.unwrap_err();
        let result = format!("{}", result);
        assert_eq!(result, error);
        _m.assert();
    }
}
