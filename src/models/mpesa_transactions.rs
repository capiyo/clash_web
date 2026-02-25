use serde::{Deserialize, Serialize};
use mongodb::bson::oid::ObjectId;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MpesaTransaction {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub paying_phone_number: String,
    pub transaction_date: String,
    pub mpesa_receipt_number: String,
    pub paid_amount: String,
    pub merchant_request_id: String,
    pub checkout_request_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateMpesaTransaction {
    pub paying_phone_number: String,
    pub transaction_date: String,
    pub mpesa_receipt_number: String,
    pub paid_amount: String,
    pub merchant_request_id: String,
    pub checkout_request_id: String,
}

#[derive(Debug, Deserialize)]
pub struct MpesaTransactionQuery {
    pub paying_phone_number: Option<String>,
    pub mpesa_receipt_number: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CallbackData {
    pub Body: CallbackBody,
}

#[derive(Debug, Deserialize)]
pub struct CallbackBody {
    pub stkCallback: StkCallback,
}

#[derive(Debug, Deserialize)]
pub struct StkCallback {
    #[serde(rename = "MerchantRequestID")]
    pub merchant_request_id: String,
    #[serde(rename = "CheckoutRequestID")]
    pub checkout_request_id: String,
    #[serde(rename = "ResultCode")]
    pub result_code: i32,
    #[serde(rename = "ResultDesc")]
    pub result_desc: String,
    #[serde(rename = "CallbackMetadata", default)]
    pub callback_metadata: Option<CallbackMetadata>,
}

#[derive(Debug, Deserialize)]
pub struct CallbackMetadata {
    pub Item: Vec<MetadataItem>,
}

#[derive(Debug, Deserialize)]
pub struct MetadataItem {
    pub Name: String,
    pub Value: serde_json::Value,
}