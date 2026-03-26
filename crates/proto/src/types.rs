use bytes::Bytes;
pub enum RespValue {
    SimpleString(Bytes),
    SimpleError(Bytes),
    Integer(i64),
    BlobString(Bytes),
    BlobError(Bytes),
    Boolean(bool),
    Double(f64),
    BigNumber(i128),
    Null,
    Array(Vec<RespValue>),
    Map(Vec<(RespValue, RespValue)>),
    Set(Vec<RespValue>),
    Push(Vec<RespValue>),
    VerbatimString { encoding: Bytes, data: Bytes },
}
