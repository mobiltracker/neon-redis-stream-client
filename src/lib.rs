use std::{error::Error, fmt::Formatter, sync::Mutex};

use neon::prelude::*;
use once_cell::sync::OnceCell;
use redis_stream_client::sync::RedisStreamClient;

pub struct RedisStreamWrapper(&'static Mutex<RedisStreamClient>);

impl Finalize for RedisStreamWrapper {}

static REDIS_CLIENT: OnceCell<redis::Client> = OnceCell::new();
static REDIS_STREAM_CLIENT: OnceCell<Mutex<RedisStreamClient>> = OnceCell::new();

#[derive(Debug)]
// All errors will be converted to JavaScript exceptions with the `Display`
// implementation as the `Error` message.
struct StreamClientError(String);

impl From<redis::RedisError> for StreamClientError {
    fn from(err: redis::RedisError) -> Self {
        StreamClientError(format!("{:?}", err))
    }
}

impl std::fmt::Display for StreamClientError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.write_str(&self.0)
    }
}

impl Error for StreamClientError {}

fn hello(mut cx: FunctionContext) -> JsResult<JsString> {
    return Ok(JsString::new(&mut cx, "Hello world"));
}

fn connect(mut cx: FunctionContext) -> JsResult<JsBox<RedisStreamWrapper>> {
    let connection_string = cx.argument::<JsString>(0)?.value(&mut cx);
    let stream_key = cx.argument::<JsString>(1)?.value(&mut cx);
    let consumer_group = cx.argument::<JsString>(2)?.value(&mut cx);
    let consumer_prefix = cx.argument::<JsString>(3)?.value(&mut cx);

    let redis_client = redis::Client::open(connection_string)
        .map_err(|err| StreamClientError::from(err))
        // Convert a Rust error to a JavaScript exception
        .or_else(|err| cx.throw_error(err.to_string()))?;

    REDIS_CLIENT.get_or_init(|| redis_client);

    let redis_connection = REDIS_CLIENT
        .get()
        .unwrap()
        .get_connection()
        .map_err(|err| StreamClientError::from(err))
        .or_else(|err| cx.throw_error(err.to_string()))?;

    let stream_key: &'static str = Box::leak(Box::new(stream_key));
    let consumer_group: &'static str = Box::leak(Box::new(consumer_group));
    let consumer_prefix: &'static str = Box::leak(Box::new(consumer_prefix));

    let redis_stream_client = RedisStreamClient::new(
        redis_connection,
        stream_key,
        &consumer_group,
        &consumer_prefix,
    )
    .map_err(|err| {
        StreamClientError(format!(
            "Failed to connect to redis stream client: {}. Key:{} group:{}, prefix:{}",
            err, stream_key, consumer_group, consumer_prefix
        ))
    })
    .or_else(|err| cx.throw_error(err.to_string()))?;

    REDIS_STREAM_CLIENT.get_or_init(|| Mutex::new(redis_stream_client));

    let wrapper = RedisStreamWrapper(REDIS_STREAM_CLIENT.get().unwrap());
    Ok(cx.boxed(wrapper))
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("connect", connect)?;
    cx.export_function("hello", hello)?;
    Ok(())
}
