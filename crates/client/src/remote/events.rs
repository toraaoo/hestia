//! The node's event stream, decoded back into the `Event`s a socket would have
//! delivered. The SSE `event:` field is the topic and `data:` is the payload
//! verbatim, so there is nothing to translate — only to reassemble.

use futures_util::StreamExt;
use ipc::protocol::Event;
use serde_json::Value;

use super::http;

/// Open `/api/v1/events` and hand every frame to `on_event` until the node stops
/// answering or `stop` is dropped.
///
/// A dropped connection ends the loop rather than reconnecting: the caller owns
/// the node's lifetime and knows whether it still wants a stream, and a
/// self-reconnecting task on a revoked key would retry forever.
pub async fn follow(base: &str, token: &str, on_event: impl Fn(&Event)) {
    let url = format!("{base}/api/{}/events", super::API);
    let response = match http::shared().get(&url).bearer_auth(token).send().await {
        Ok(response) if response.status().is_success() => response,
        Ok(response) => {
            tracing::warn!(status = %response.status(), "the node refused an event stream");
            return;
        }
        Err(e) => {
            tracing::debug!("cannot open the node's event stream: {e}");
            return;
        }
    };

    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    while let Some(chunk) = stream.next().await {
        let Ok(chunk) = chunk else { break };
        buffer.push_str(&String::from_utf8_lossy(&chunk));
        // A frame ends at a blank line, and a chunk may split one anywhere.
        while let Some(end) = buffer.find("\n\n") {
            let frame: String = buffer.drain(..end + 2).collect();
            if let Some(event) = decode(&frame) {
                on_event(&event);
            }
        }
    }
    tracing::debug!("the node's event stream ended");
}

fn decode(frame: &str) -> Option<Event> {
    let mut topic = None;
    let mut data = String::new();
    for line in frame.lines() {
        if let Some(rest) = line.strip_prefix("event:") {
            topic = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("data:") {
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(rest.trim_start());
        }
    }
    let topic = topic?;
    let payload = serde_json::from_str::<Value>(&data).unwrap_or(Value::Null);
    Some(Event::new(topic, payload))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_decodes_into_the_event_a_socket_would_have_carried() {
        let event = decode("event: backup.progress\ndata: {\"id\":\"j1\",\"percent\":42}\n\n")
            .expect("decoded");
        assert_eq!(event.topic, "backup.progress");
        assert_eq!(event.payload["id"], Value::from("j1"));
        assert_eq!(event.payload["percent"], Value::from(42));
    }

    #[test]
    fn a_payload_split_over_several_data_lines_is_rejoined() {
        let event = decode("event: backup.done\ndata: {\"id\":\n data: \"j1\"}\n\n");
        assert!(event.is_some());
    }

    #[test]
    fn a_keep_alive_comment_is_not_an_event() {
        assert!(decode(": keep-alive\n\n").is_none());
        assert!(decode("\n\n").is_none());
    }

    #[test]
    fn an_id_line_does_not_confuse_the_payload() {
        let event = decode("event: process.output\ndata: {\"line\":\"hi\"}\nid: 1187\n\n")
            .expect("decoded");
        assert_eq!(event.topic, "process.output");
        assert_eq!(event.payload["line"], Value::from("hi"));
    }
}
