use chrono::{Duration, Utc};
use sqlx::types::BigDecimal;
use std::str::FromStr;
use std::sync::Arc;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::error::PaymentError;
use crate::payment::repo::{PaymentRepo, PaymentRow};
use crate::pb::payment_service_server::PaymentService;
use crate::pb::*;
use crate::state::AppState;

pub struct PaymentSvc {
    pub state: Arc<AppState>,
}

impl PaymentSvc {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    fn to_proto(r: PaymentRow) -> PaymentResponse {
        PaymentResponse {
            id: r.id.to_string(),
            order_id: r.order_id.to_string(),
            status: r.status,
            amount: r.amount.to_string(),
            currency: r.currency,
            gateway_reference: r.gateway_reference.unwrap_or_default(),
            payment_url: r.payment_url.unwrap_or_default(),
            created_at: r.created_at.to_rfc3339(),
            updated_at: r.updated_at.to_rfc3339(),
            failure_reason: r.failure_reason,
        }
    }
}

#[tonic::async_trait]
impl PaymentService for PaymentSvc {
    async fn create_payment(
        &self,
        req: Request<CreatePaymentRequest>,
    ) -> Result<Response<PaymentResponse>, Status> {
        let r = req.into_inner();
        //

        let amount =
            BigDecimal::from_str(&r.amount).map_err(|_| Status::invalid_argument("bad amount"))?;

        // 1) insert pending
        let now = Utc::now();
        let idem = if r.idempotency_key.is_empty() {
            format!("order:{}", r.order_id.clone())
        } else {
            r.idempotency_key.clone()
        };

        let row = PaymentRow {
            id: Uuid::now_v7().to_string(),
            order_id: r.order_id.clone(),
            idempotency_key: idem.clone(),
            amount: amount.clone(),
            currency: r.currency.clone(),
            status: "pending".into(),
            gateway: self.state.gateway.name().to_string(),
            gateway_reference: None,
            payment_url: None,
            failure_reason: None,
            customer_id: if r.customer_id.is_empty() {
                None
            } else {
                Some(r.customer_id.clone())
            },
            metadata: serde_json::to_value(r.metadata).unwrap_or_default(),
            reservation_expires_at: now + Duration::seconds(self.state.config.reservation_ttl_secs),
            created_at: now,
            updated_at: now,
        };

        // idempotency: if key exists, return existing
        if let Some(existing) =
            sqlx::query_as::<_, PaymentRow>("SELECT * FROM payments WHERE idempotency_key = $1")
                .bind(&idem)
                .fetch_optional(&self.state.db)
                .await
                .map_err(|e| PaymentError::Database(e))?
        {
            if existing.amount != amount || existing.order_id != r.order_id.clone() {
                return Err(PaymentError::IdempotencyMismatch.into());
            }
            return Ok(Response::new(Self::to_proto(existing)));
        }

        PaymentRepo::insert(&self.state.db, &row).await?;

        // 2) call gateway
        let gw = self.state.gateway.clone();
        let order_id_s = r.order_id.clone();
        let amount_s = amount.to_string();
        let currency_s = r.currency.clone();

        match gw.create(&order_id_s, &amount_s, &currency_s).await {
            Ok((reference, url)) => {
                sqlx::query(
                    "INSERT INTO payment_attempts (payment_id, kind, request, response, success)
                     VALUES ($1,'create',$2,$3,true)",
                )
                .bind(&row.id)
                .bind(serde_json::json!({"order_id": order_id_s, "amount": amount_s}))
                .bind(serde_json::json!({"reference": reference, "url": url}))
                .execute(&self.state.db)
                .await
                .map_err(|e| PaymentError::Database(e));

                // 3) processing + outbox payment.created
                let row = PaymentRepo::transition(
                    &self.state.db,
                    row.id.clone(),
                    &["pending"],
                    "processing",
                    Some(&reference),
                    Some(&url),
                    None,
                    "payment.created",
                    serde_json::json!({
                        "payment_id": row.id,
                        "order_id": row.order_id,
                        "amount": amount_s,
                        "currency": currency_s,
                    }),
                )
                .await?;
                Ok(Response::new(Self::to_proto(row)))
            }
            Err(err) => {
                sqlx::query(
                    "INSERT INTO payment_attempts (payment_id, kind, request, success, error)
                     VALUES ($1,'create',$2,false,$3)",
                )
                .bind(&row.id)
                .bind(serde_json::json!({"order_id": order_id_s, "amount": amount_s}))
                .bind(&err)
                .execute(&self.state.db)
                .await
                .map_err(|e| PaymentError::Database(e));

                let row = PaymentRepo::transition(
                    &self.state.db,
                    row.id.clone(),
                    &["pending"],
                    "failed",
                    None,
                    None,
                    Some(&err),
                    "payment.failed",
                    serde_json::json!({"payment_id": row.id, "reason": err}),
                )
                .await?;
                self.state
                    .try_publish(
                        "payment.created",
                        row.id.clone(),
                        Some(row.order_id.clone()),
                        &row.status,
                        None,
                        serde_json::json!({"amount": amount_s, "currency": currency_s}),
                    )
                    .await;
                Ok(Response::new(Self::to_proto(row)))
            }
        }
    }

    async fn get_payment(
        &self,
        req: Request<GetPaymentRequest>,
    ) -> Result<Response<PaymentResponse>, Status> {
        let row = PaymentRepo::get(&self.state.db, req.into_inner().payment_id).await?;
        Ok(Response::new(Self::to_proto(row)))
    }

    async fn cancel_payment(
        &self,
        req: Request<CancelPaymentRequest>,
    ) -> Result<Response<PaymentResponse>, Status> {
        let r = req.into_inner();

        if let Ok(row) = PaymentRepo::get(&self.state.db, r.payment_id.clone()).await {
            if let Some(reference) = &row.gateway_reference {
                let _ = self.state.gateway.cancel(reference).await;
            }
        }

        let row = PaymentRepo::transition(
            &self.state.db,
            r.payment_id.clone(),
            &["pending", "processing"],
            "cancelled",
            None,
            None,
            Some(&r.reason),
            "payment.cancelled",
            serde_json::json!({"payment_id": &r.payment_id, "reason": r.reason}),
        )
        .await?;
        Ok(Response::new(Self::to_proto(row)))
    }

    async fn refund_payment(
        &self,
        req: Request<RefundPaymentRequest>,
    ) -> Result<Response<PaymentResponse>, Status> {
        let r = req.into_inner();
        let id = &r.payment_id;
        let row = PaymentRepo::transition(
            &self.state.db,
            id.to_string(),
            &["succeeded"],
            "refunded",
            None,
            None,
            None,
            "payment.refunded",
            serde_json::json!({"payment_id": id, "amount": r.amount, "reason": r.reason}),
        )
        .await?;
        Ok(Response::new(Self::to_proto(row)))
    }
}
