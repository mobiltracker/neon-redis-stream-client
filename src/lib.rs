use std::{error::Error, fmt::Formatter, sync::Mutex};

use neon::prelude::*;
use once_cell::sync::OnceCell;
use redis::FromRedisValue;
use redis_stream_client::{sync::RedisStreamClient, sync::RedisStreamMessage};

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

fn read_next(mut cx: FunctionContext) -> JsResult<JsPromise> {
    let promise = cx
        .task(
            move || -> Result<Option<RedisStreamMessage>, StreamClientError> {
                let stream_client = REDIS_STREAM_CLIENT.get().ok_or_else(|| {
                    StreamClientError(
            "Stream client disconnected. You might have forgotten to call connect() at some point"
                .to_string(),
        )
                })?;

                let mut stream_client = stream_client
                    .lock()
                    .map_err(|_| StreamClientError("Poisoned Mutex".to_owned()))?;

                stream_client
                    .read_next()
                    .map_err(|err| StreamClientError::from(err))
            },
        )
        .promise(resolve_get_next);

    Ok(promise)
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("connect", connect)?;
    cx.export_function("readNext", read_next)?;
    cx.export_function("hello", hello)?;
    Ok(())
}

fn resolve_get_next<'a>(
    cx: &mut TaskContext<'a>,
    // Return value from `cx.task(..)` closure
    result: Result<Option<RedisStreamMessage>, StreamClientError>,
) -> JsResult<'a, JsObject> {
    let output = result
        // An error may have occurred while compressing; conditionally grab the
        // written data
        .and_then(|msg| {
            if let Some(msg) = msg {
                let msg_object = JsObject::new(cx);

                for (key, val) in msg.inner_map {
                    let key = JsString::new(cx, key);
                    let val = JsString::new(cx, String::from_redis_value(&val).unwrap());
                    msg_object
                        .set(cx, key, val)
                        .map_err(|_| StreamClientError("Error convering to obj".to_string()))?;
                }

                Ok(msg_object)
            } else {
                Ok(JsObject::new(cx))
            }
        })
        // Convert a Rust error to a JavaScript exception
        .or_else(|err| cx.throw_error(err.to_string()))?;

    Ok(output)
}
