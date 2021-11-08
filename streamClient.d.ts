declare const streamFFI: any;
declare type Message = {
    key: string;
} & Object;
declare class RedisStreamClient {
    private clientRef;
    private connectionString;
    private groupName;
    private consumerPrefix;
    private streamKey;
    constructor(connectionString: string, streamKey: string, groupName: string, consumerPrefix: string);
    connect(): void;
    readNext(): Promise<Message>;
    ackMessageId(msgId: string): Promise<any>;
}
