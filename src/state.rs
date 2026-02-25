use mongodb::Database;
use std::sync::Arc;

use crate::services::cloudinary::CloudinaryService;
use crate::services::mpesa_service::MpesaService;
use crate::services::fcm_service::FCMService;
use crate::services::otp_service::OTPService;
use crate::services::sms_service::SMSService;
use crate::errors::AppError;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub mpesa_service: Option<Arc<MpesaService>>,
    pub fcm_service: Option<Arc<FCMService>>,
    pub cloudinary: CloudinaryService,
    pub otp_service: OTPService,
    pub sms_service: SMSService,
}

#[derive(Clone)]
pub struct SmsConfig {
    pub api_key: String,
    pub username: String,
    pub from: String,
}

impl AppState {
    pub fn new(
        db: Database,
        jwt_secret: String,
        sms_config: SmsConfig,
    ) -> Result<Self, AppError> {
        let cloudinary = CloudinaryService::new()?;

        let otp_service = OTPService::new(db.clone(), jwt_secret);
        let sms_service = SMSService::new(
            sms_config.api_key,
            sms_config.username,
            sms_config.from,
        );

        Ok(AppState {
            db,
            mpesa_service: None,
            fcm_service: None,
            cloudinary,
            otp_service,
            sms_service,
        })
    }

    pub fn with_mpesa(mut self, mpesa_service: Arc<MpesaService>) -> Self {
        self.mpesa_service = Some(mpesa_service);
        self
    }

    pub fn with_fcm(mut self, fcm_service: Arc<FCMService>) -> Self {
        self.fcm_service = Some(fcm_service);
        self
    }
}
