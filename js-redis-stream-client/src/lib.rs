use std::sync::Mutex;

use neon::{prelude::*, result::JsResultExt, result::Throw};
use once_cell::sync::{Lazy, OnceCell};
use redis_stream_client::sync::RedisStreamClient;

pub struct RedisStreamWrapper(&'static Mutex<RedisStreamClient>);

impl Finalize for RedisStreamWrapper {}

static REDIS_CLIENT: OnceCell<redis::Client> = OnceCell::new();
static REDIS_STREAM_CLIENT: OnceCell<Mutex<RedisStreamClient>> = OnceCell::new();

fn connect(mut cx: FunctionContext) -> JsResult<JsBox<RedisStreamWrapper>> {
    let connection_string = cx.argument::<JsString>(0)?.value(&mut cx);
    let stream_key = cx.argument::<JsString>(1)?.value(&mut cx);
    let consumer_group = cx.argument::<JsString>(2)?.value(&mut cx);
    let consumer_prefix = cx.argument::<JsString>(3)?.value(&mut cx);

    let redis_client = redis::Client::open(connection_string);

    if redis_client.is_err() {
        let error = JsError::error(&mut cx, format!("{}", redis_client.unwrap_err()))?;
        return cx.throw(error);
    };

    REDIS_CLIENT.get_or_init(|| redis_client.unwrap());

    let redis_connection = REDIS_CLIENT.get().unwrap().get_connection();
    if redis_connection.is_err() {
        let error = JsError::error(&mut cx, "")?;
        return cx.throw(error);
    }

    let stream_key: &'static str = Box::leak(Box::new(stream_key));
    let consumer_group: &'static str = Box::leak(Box::new(consumer_group));
    let consumer_prefix: &'static str = Box::leak(Box::new(consumer_prefix));

    let redis_stream_client = RedisStreamClient::new(
        redis_connection.unwrap(),
        stream_key,
        &consumer_group,
        &consumer_prefix,
    );

    if redis_stream_client.is_err() {
        let error = JsError::error(&mut cx, "")?;
        return cx.throw(error);
    }

    let redis_stream_client = redis_stream_client.unwrap();

    REDIS_STREAM_CLIENT.get_or_init(|| Mutex::new(redis_stream_client));

    let wrapper = RedisStreamWrapper(REDIS_STREAM_CLIENT.get().unwrap());
    Ok(cx.boxed(wrapper))
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("connect", connect)?;
    Ok(())
}
