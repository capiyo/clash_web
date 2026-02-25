// config.rs
use dotenv::dotenv;
use std::env;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub mpesa_consumer_key: String,
    pub mpesa_consumer_secret: String,
    pub mpesa_short_code: String,
    pub mpesa_passkey: String,
    pub mpesa_callback_url: String,
    pub mpesa_b2c_result_url: String,
    pub mpesa_b2c_queue_timeout_url: String,
    pub mpesa_initiator_name: String,
    pub mpesa_security_credential: String,
    pub mpesa_environment: String,
    pub jwt_secret: String,
    pub database_url: String,
    pub port: u16,
    pub host: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        dotenv().ok();

        let mpesa_environment = env::var("MPESA_ENVIRONMENT")
            .unwrap_or_else(|_| "sandbox".to_string());

        let is_production = mpesa_environment == "production";

        // Log environment for debugging
        println!("==========================================");
        println!("MPESA ENVIRONMENT: {}", mpesa_environment);
        println!("IS PRODUCTION: {}", is_production);
        println!("==========================================");

        AppConfig {
            mpesa_consumer_key: env::var("MPESA_CONSUMER_KEY")
                .expect("MPESA_CONSUMER_KEY must be set"),
            mpesa_consumer_secret: env::var("MPESA_CONSUMER_SECRET")
                .expect("MPESA_CONSUMER_SECRET must be set"),
            mpesa_short_code: env::var("MPESA_SHORT_CODE")
                .expect("MPESA_SHORT_CODE must be set"),
            mpesa_passkey: env::var("MPESA_PASSKEY")
                .expect("MPESA_PASSKEY must be set"),
            mpesa_callback_url: env::var("MPESA_CALLBACK_URL")
                .expect("MPESA_CALLBACK_URL must be set"),
            mpesa_b2c_result_url: env::var("MPESA_B2C_RESULT_URL")
                .expect("MPESA_B2C_RESULT_URL must be set"),
            mpesa_b2c_queue_timeout_url: env::var("MPESA_B2C_QUEUE_TIMEOUT_URL")
                .expect("MPESA_B2C_QUEUE_TIMEOUT_URL must be set"),
            mpesa_initiator_name: env::var("MPESA_INITIATOR_NAME")
                .expect("MPESA_INITIATOR_NAME must be set"),
            mpesa_security_credential: env::var("MPESA_SECURITY_CREDENTIAL")
                .expect("MPESA_SECURITY_CREDENTIAL must be set"),
            mpesa_environment,
            jwt_secret: env::var("JWT_SECRET")
                .expect("JWT_SECRET must be set"),
            database_url: env::var("DATABASE_URL")
                .expect("DATABASE_URL must be set"),
            port: env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .expect("PORT must be a number"),
            host: env::var("HOST")
                .unwrap_or_else(|_| "0.0.0.0".to_string()),
        }
    }

    pub fn get_mpesa_urls(&self) -> (String, String, String) {
        let base_url = if self.mpesa_environment == "production" {
            "https://api.safaricom.co.ke"
        } else {
            "https://sandbox.safaricom.co.ke"
        };

        println!("[CONFIG] Using M-Pesa Base URL: {}", base_url);
        println!("[CONFIG] Business Shortcode: {}", self.mpesa_short_code);
        println!("[CONFIG] Initiator: {}", self.mpesa_initiator_name);
        println!("[CONFIG] Callback URL: {}", self.mpesa_b2c_result_url);

        let auth_url = format!("{}/oauth/v1/generate?grant_type=client_credentials", base_url);
        let stk_url = format!("{}/mpesa/stkpush/v1/processrequest", base_url);
        let b2c_url = format!("{}/mpesa/b2c/v1/paymentrequest", base_url);

        (auth_url, stk_url, b2c_url)
    }

    pub fn is_production(&self) -> bool {
        self.mpesa_environment == "production"
    }

    pub fn get_config_info(&self) -> serde_json::Value {
        serde_json::json!({
            "environment": self.mpesa_environment,
            "is_production": self.is_production(),
            "business_shortcode": self.mpesa_short_code,
            "initiator_name": self.mpesa_initiator_name,
            "callback_url": self.mpesa_callback_url,
            "b2c_result_url": self.mpesa_b2c_result_url,
            "b2c_timeout_url": self.mpesa_b2c_queue_timeout_url,
            "consumer_key_set": !self.mpesa_consumer_key.is_empty(),
            "consumer_secret_set": !self.mpesa_consumer_secret.is_empty(),
            "security_credential_length": self.mpesa_security_credential.len(),
            "port": self.port,
            "host": self.host,
        })
    }
}