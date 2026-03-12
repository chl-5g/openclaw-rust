use serde::{de::DeserializeOwned, Serialize};
use std::marker::PhantomData;

pub trait WsMessageCodec: Clone + Send + Sync + 'static {
    type Message: Clone + Send + Sync + Serialize + DeserializeOwned + 'static;

    fn encode(&self, msg: &Self::Message) -> Vec<u8> {
        serde_json::to_vec(msg).unwrap_or_default()
    }

    fn decode(&self, data: &[u8]) -> Option<Self::Message> {
        serde_json::from_slice(data).ok()
    }
}

#[derive(Clone)]
pub struct JsonCodec<M: Clone + Send + Sync + Serialize + DeserializeOwned + 'static> {
    _marker: PhantomData<M>,
}

impl<M: Clone + Send + Sync + Serialize + DeserializeOwned + 'static> JsonCodec<M> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<M: Clone + Send + Sync + Serialize + DeserializeOwned + 'static> Default for JsonCodec<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: Clone + Send + Sync + Serialize + DeserializeOwned + 'static> WsMessageCodec for JsonCodec<M> {
    type Message = M;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
    struct TestMessage {
        id: String,
        content: String,
    }

    #[test]
    fn test_json_codec_encode_decode() {
        let codec = JsonCodec::<TestMessage>::new();
        let msg = TestMessage {
            id: "test-1".to_string(),
            content: "Hello".to_string(),
        };

        let encoded = codec.encode(&msg);
        let decoded = codec.decode(&encoded);

        assert!(decoded.is_some());
        assert_eq!(decoded.unwrap(), msg);
    }

    #[test]
    fn test_json_codec_decode_invalid() {
        let codec = JsonCodec::<TestMessage>::new();
        let invalid_data = b"not valid json";

        let decoded = codec.decode(invalid_data);
        assert!(decoded.is_none());
    }
}
