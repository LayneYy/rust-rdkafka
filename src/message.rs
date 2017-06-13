//! Store and manipulate Kafka messages.
use rdsys;
use rdsys::types::*;

use std::ffi::CStr;
use std::slice;
use std::str;

/// Timestamp of a message
#[derive(Debug,PartialEq,Eq,Clone,Copy)]
pub enum Timestamp {
    NotAvailable,
    CreateTime(i64),
    LogAppendTime(i64)
}

/// A native librdkafka message.
#[derive(Debug)]
pub struct Message {
    ptr: *mut RDKafkaMessage,
}

unsafe impl Send for Message {}

impl Message {
    /// Creates a new Message that wraps the native Kafka message pointer.
    pub fn new(ptr: *mut RDKafkaMessage) -> Message {
        Message { ptr: ptr }
    }

    /// Returns a pointer to the RDKafkaMessage.
    pub fn ptr(&self) -> *mut RDKafkaMessage {
        self.ptr
    }

    /// Returns a pointer to the message's RDKafkaTopic
    pub fn topic_ptr(&self) -> *mut RDKafkaTopic {
        unsafe { (*self.ptr).rkt }
    }

    /// Returns the length of the key field of the message.
    pub fn key_len(&self) -> usize {
        unsafe { (*self.ptr).key_len }
    }

    /// Returns the length of the payload field of the message.
    pub fn payload_len(&self) -> usize {
        unsafe { (*self.ptr).len }
    }

    /// Returns the key of the message, or None if there is no key.
    pub fn key(&self) -> Option<&[u8]> {
        unsafe {
            if (*self.ptr).key.is_null() {
                None
            } else {
                Some(slice::from_raw_parts::<u8>((*self.ptr).key as *const u8, (*self.ptr).key_len))
            }
        }
    }

    /// Returns the payload of the message, or None if there is no payload.
    pub fn payload(&self) -> Option<&[u8]> {
        unsafe {
            if (*self.ptr).payload.is_null() {
                None
            } else {
                Some(slice::from_raw_parts::<u8>((*self.ptr).payload as *const u8, (*self.ptr).len))
            }
        }
    }

    /// Returns the source topic of the message.
    pub fn topic_name(&self) -> &str {
         unsafe {
             CStr::from_ptr(rdsys::rd_kafka_topic_name((*self.ptr).rkt))
                 .to_str()
                 .expect("Topic name is not valid UTF-8")
         }
     }

    /// Converts the raw bytes of the payload to a reference of type &P, pointing to the same data inside
    /// the message. The returned reference cannot outlive the message.
    pub fn payload_view<P: ?Sized + FromBytes>(&self) -> Option<Result<&P, P::Error>> {
        self.payload().map(P::from_bytes)
    }

    /// Converts the raw bytes of the key to a reference of type &K, pointing to the same data inside
    /// the message. The returned reference cannot outlive the message.
    pub fn key_view<K: ?Sized + FromBytes>(&self) -> Option<Result<&K, K::Error>> {
        self.key().map(K::from_bytes)
    }

    /// Returns the partition number where the message is stored.
    pub fn partition(&self) -> i32 {
        unsafe { (*self.ptr).partition }
    }

    /// Returns the offset of the message.
    pub fn offset(&self) -> i64 {
        unsafe { (*self.ptr).offset }
    }

    /// Returns the message timestamp for a consumed message if available.
    pub fn timestamp(&self) -> Timestamp {
        let mut timestamp_type = rdsys::rd_kafka_timestamp_type_t::RD_KAFKA_TIMESTAMP_NOT_AVAILABLE;
        let timestamp = unsafe {
            rdsys::rd_kafka_message_timestamp(
                self.ptr,
                &mut timestamp_type
            )

        };
        match timestamp_type {
            rdsys::rd_kafka_timestamp_type_t::RD_KAFKA_TIMESTAMP_NOT_AVAILABLE => Timestamp::NotAvailable,
            rdsys::rd_kafka_timestamp_type_t::RD_KAFKA_TIMESTAMP_CREATE_TIME => Timestamp::CreateTime(timestamp),
            rdsys::rd_kafka_timestamp_type_t::RD_KAFKA_TIMESTAMP_LOG_APPEND_TIME => Timestamp::LogAppendTime(timestamp)
        }
    }
}

impl Drop for Message {
    fn drop(&mut self) {
        trace!("Destroying message {:?}", self);
        unsafe { rdsys::rd_kafka_message_destroy(self.ptr) };
    }
}

/// Given a reference to a byte array, returns a different view of the same data.
/// No copy of the data should be performed.
pub trait FromBytes {
    type Error;
    fn from_bytes(&[u8]) -> Result<&Self, Self::Error>;
}

impl FromBytes for [u8] {
    type Error = ();
    fn from_bytes(bytes: &[u8]) -> Result<&Self, Self::Error> {
        Ok(bytes)
    }
}

impl FromBytes for str {
    type Error = str::Utf8Error;
    fn from_bytes(bytes: &[u8]) -> Result<&Self, Self::Error> {
        str::from_utf8(bytes)
    }
}

/// Given some data, returns the byte representation of that data.
/// No copy of the data should be performed.
pub trait ToBytes {
    fn to_bytes(&self) -> &[u8];
}

impl ToBytes for [u8] {
    fn to_bytes(&self) -> &[u8] {
        self
    }
}

impl ToBytes for str {
    fn to_bytes(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl ToBytes for Vec<u8> {
    fn to_bytes(&self) -> &[u8] {
        self.as_slice()
    }
}

impl ToBytes for String {
    fn to_bytes(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<'a, T: ToBytes> ToBytes for &'a T {
    fn to_bytes(&self) -> &[u8] {
        (*self).to_bytes()
    }
}

impl ToBytes for () {
    fn to_bytes(&self) -> &[u8] {
        &[]
    }
}
