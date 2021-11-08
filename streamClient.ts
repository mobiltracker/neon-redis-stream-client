"use strict";
/// @ts-ignore
import { connect, readNext, ackMessage } from "./index.node";

type Message = { key: string } & Object;

class RedisStreamClient {
  private clientRef: any;
  private connectionString: string;
  private groupName: string;
  private consumerPrefix: string;
  private streamKey: string;

  constructor(
    connectionString: string,
    streamKey: string,
    groupName: string,
    consumerPrefix: string
  ) {
    this.connectionString = connectionString;
    this.streamKey = streamKey;
    this.groupName = groupName;
    this.consumerPrefix = consumerPrefix;
    this.clientRef = null;
  }

  connect() {
    // //"redis://127.0.0.1",
    // "test-stream",
    // "test-group",
    // "test-consumer"
    const client = connect(
      this.connectionString,
      this.streamKey,
      this.groupName,
      this.consumerPrefix
    );

    this.clientRef = client;
  }

  async readNext() {
    const msg: Object = await readNext(this.clientRef);
    return msg;
  }

  async ackMessage(msgId: string) {
    return await ackMessage(this.clientRef, msgId);
  }
}

export default { connect, readNext, ackMessage };
