use bytes::Bytes;
#[derive(Debug)]
pub enum RespValue {
    SimpleString(Bytes),
    SimpleError(Bytes),
    Integer(i64),
    BlobString(Bytes),
    Null,
    Boolean(bool),
    Array(Vec<RespValue>),
    Double(f64),
    BlobError(Bytes),
    BigNumber(i128),
    Map(Vec<(RespValue, RespValue)>),
    Set(Vec<RespValue>),
    Push(Vec<RespValue>),
    VerbatimString { encoding: Bytes, data: Bytes },
}
