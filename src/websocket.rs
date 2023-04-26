use futures_util::{SinkExt, StreamExt};

use base64::{engine::general_purpose, Engine as _};

use rand::{self, Rng};
use tokio_tungstenite::tungstenite::handshake::client::Request;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::connect_async;
use uuid::Uuid;

use anyhow::Result;
use chrono::Local;

// type WSStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

const SPEECH_API: &str = "wss://speech.platform.bing.com/consumer/speech/synthesize/readaloud/edge/v1?TrustedClientToken=6A5AA1D4EAFF4E9FB37E23D68491D6F4&ConnectionId=";
// pub const VOICE_XIAOXIAO: &str =
    // "Microsoft Server Speech Text to Speech Voice (zh-CN, XiaoxiaoNeural)";
pub const VOICE_XIAOYI: &str = "Microsoft Server Speech Text to Speech Voice (zh-CN, XiaoyiNeural)";

pub struct Speaker {
    pub voice: String,
    volume: String,
    rate: String,
}

impl Speaker {
    pub fn new(voice: &str, volume: &str, rate: &str) -> Speaker {
        let speaker = Self {
            voice: String::from(voice),
            volume: String::from(volume),
            rate: String::from(rate),
        };

        speaker
    }

    pub async fn say(self, ctx: &str) -> Result<Vec<u8>> {
        let req = Self::make_req()?;

        log::debug!("Starting connect websocket.");
        let (ws, _) = connect_async(req).await?;
        log::debug!("Connect websocket success!");

        let (mut write, mut read) = ws.split();

        let bin_data = tokio::spawn(async move {
            log::debug!("Starting new thread to deal websocket io.");
            let mut bin_data: Vec<u8> = Vec::new();
            let pattern = b"Path:audio\r\n";
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(m) => match m {
                        Message::Text(text) => {
                            let find_end = text.find("turn.end");
                            if find_end.is_some() {
                                break;
                            }
                        }
                        Message::Binary(bin) => {
                            let pos = bin
                                .windows(pattern.len())
                                .position(|window| window == pattern);
                            match pos {
                                Some(p) => bin_data.extend(&bin[(p + 12)..]),
                                None => log::error!("Receive binary data have some mistake.")
                            }
                        }
                        _ => {}
                    },
                    Err(e) => log::error!("An error occurred: {}", e),
                }
            }

            log::debug!("Websocket connect stopped.");

            bin_data
        });

        let header_str = Self::make_header_str();
        let send_header = Message::text(&header_str);

        write.send(send_header).await?;
        log::debug!("Header message sent.");
        write
            .send(Message::text(&self.ssml_header_and_data(
                &Self::connect_id(),
                &Self::date_to_string(),
                &Self::remove_incompatible_characters(ctx),
            )))
            .await?;

        log::debug!("Main content sent.");
        let bin_data = bin_data.await?;

        write
            .send(Message::Close(Some(CloseFrame {
                code: CloseCode::Away,
                reason: "Away".into(),
            })))
            .await?;
        write.close().await?;
        log::debug!("Close message sent.");
        Ok(bin_data)
    }

    fn ssml_header_and_data(&self, request_id: &str, date: &str, text: &str) -> String {
        let mut data = String::new();
        data.push_str(&format!("X-RequestId:{}\r\n", request_id));
        data.push_str("Content-Type:application/ssml+xml\r\n");
        data.push_str(&format!("X-Timestamp:{}Z\r\n", date));
        data.push_str("Path:ssml\r\n\r\n");
        data.push_str(
            "<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis' xml:lang='en-US'>",
        );
        data.push_str(&format!(
            "<voice name='{}'><prosody pitch='+0Hz' rate='{}' volume='{}'>",
            self.voice, self.rate, self.volume
        ));
        data.push_str(&format!("{}</prosody></voice></speak>", text));
        data
    }

    fn date_to_string() -> String {
        let cst = Local::now();
        let formatted = cst.format("%a %b %d %Y %H:%M:%S GMT+0800 (中国标准时间)");
        formatted.to_string()
    }

    fn connect_id() -> String {
        Uuid::new_v4().to_string().replacen("-", "", 4)
    }

    fn make_req() -> Result<Request> {
        let url = format!("{}{}", SPEECH_API, Self::connect_id());

        let mut rng = rand::thread_rng();
        let bytes: [u8; 16] = rng.gen();
        let base64_str = general_purpose::STANDARD.encode(bytes);
        let req = Request::builder().uri(url)
        .header("Host", "speech.platform.bing.com")
        .header("Connection", "Upgrade")
        .header("Pragma", "no-cache")
        .header("Upgrade", "websocket")
        .header("Cache-Control", "no-cache")
        .header("Origin", "chrome-extension://jdiccldimpdaibmpdkjnbmckianbfold")
        .header("Accept-Encoding", "gzip, deflate, br")
        .header("Accept-Language", "zh-CN,zh;q=0.9")
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.77 Safari/537.36 Edg/91.0.864.41")
        .header("Sec-WebSocket-Version", 13)
        .header("Sec-WebSocket-Extensions", "permessage-deflate; client_max_window_bits")
        .header("Sec-WebSocket-Key", base64_str)
        .body(())?;

        Ok(req)
    }

    fn make_header_str() -> String {
        let mut header_str = String::new();
        header_str.push_str(&format!("X-Timestamp:{}\r\n", Self::date_to_string()));
        header_str.push_str("Content-Type:application/json; charset=utf-8\r\n");
        header_str.push_str("Path:speech.config\r\n\r\n");
        header_str.push_str(r#"{"context":{"synthesis":{"audio":{"metadataoptions":{"#);
        header_str.push_str(r#""sentenceBoundaryEnabled":false,"wordBoundaryEnabled":true},"#);
        header_str.push_str(r#""outputFormat":"audio-24khz-48kbitrate-mono-mp3""#);
        header_str.push_str("}}}}\r\n");
        header_str
    }

    fn remove_incompatible_characters(text: &str) -> String {
        let mut result = String::new();
        for c in text.chars() {
            if !c.is_ascii_control() || ['&', '#', '$', '¥'].contains(&c) {
                result.push(c);
            }
        }
        result
    }
}
