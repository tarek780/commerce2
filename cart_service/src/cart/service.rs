

use chrono::Utc;

use crate::cart::model::{AddItemRequest, CachedCart, CartItem};
use crate::cart::repository::{GuestCartRepo, UserCartRepo};
use crate::error::CartError;

pub struct CartService {
    pub guest_repo: GuestCartRepo,
    pub user_repo: UserCartRepo,
}

impl CartService {
    // ---------- Guest ----------
    pub async fn create_guest_cart(&self) -> Result<String, CartError> {
        self.guest_repo.create().await
    }

    pub async fn get_guest_cart(&self, guest_id: &str) -> Result<CachedCart, CartError> {
        self.guest_repo.get(guest_id).await
    }

    pub async fn add_guest_item(
        &self,
        guest_id: &str,
        req: AddItemRequest,
    ) -> Result<(), CartError> {
        if req.quantity <= 0 {
            return Err(CartError::InvalidQuantity);
        }
        let mut cart = self.guest_repo.get(guest_id).await?;
        match cart.items.iter_mut().find(|i| i.sku_id == req.sku_id) {
            Some(item) => {
                item.quantity = item.quantity.saturating_add(req.quantity);
                item.price_at_add = req.price;
            }
            None => cart.items.push(CartItem {
                sku_id: req.sku_id,
                quantity: req.quantity,
                price_at_add: req.price,
                added_at: Utc::now(),
            }),
        }
        cart.updated_at = Utc::now();
        self.guest_repo.put(guest_id, &cart).await
    }

    // ---------- User ----------
    pub async fn get_user_cart(&self, user_id: &str) -> Result<CachedCart, CartError> {
        if let Some(cached) = self.user_repo.get_cached(user_id).await? {
            tracing::debug!(user_id, "user cart from redis");
            return Ok(cached);
        }
        let cart_id = self.user_repo.get_or_create_cart_id(user_id).await?;
        let items = self.user_repo.load_items(&cart_id).await?;
        let cart = CachedCart {
            items,
            updated_at: Utc::now(),
        };
        self.user_repo.put_cached(user_id, &cart).await?;
        tracing::debug!(user_id, "user cart loaded from DB and cached");
        Ok(cart)
    }

    pub async fn add_user_item(&self, user_id: &str, req: AddItemRequest) -> Result<(), CartError> {
        if req.quantity <= 0 {
            return Err(CartError::InvalidQuantity);
        }
        let cart_id = self.user_repo.get_or_create_cart_id(user_id).await?;
        self.user_repo.upsert_items_bulk(&cart_id, &[req]).await?;
        self.refresh_user_cache(user_id, &cart_id).await?;
        Ok(())
    }

    pub async fn add_user_items(
        &self,
        user_id: &str,
        items: Vec<AddItemRequest>,
    ) -> Result<(), CartError> {
        if items.is_empty() {
            return Ok(());
        }
        if items.iter().any(|i| i.quantity <= 0) {
            return Err(CartError::InvalidQuantity);
        }
        let cart_id = self.user_repo.get_or_create_cart_id(user_id).await?;
        self.user_repo.upsert_items_bulk(&cart_id, &items).await?;
        self.refresh_user_cache(user_id, &cart_id).await?;
        Ok(())
    }

    // ---------- Merge ---------
    pub async fn merge_guest_into_user(
        &self,
        user_id: &str,
        guest_id: &str,
    ) -> Result<(), CartError> {
        let guest_cart = self.guest_repo.get(guest_id).await?;

        if guest_cart.items.is_empty() {
            self.guest_repo.delete(guest_id).await?;
            return Ok(());
        }

        let cart_id = self.user_repo.get_or_create_cart_id(user_id).await?;

        let requests: Vec<AddItemRequest> = guest_cart
            .items
            .iter()
            .map(|i| AddItemRequest {
                sku_id: i.sku_id.clone(),
                quantity: i.quantity,
                price: i.price_at_add,
            })
            .collect();

        self.user_repo
            .upsert_items_bulk(&cart_id, &requests)
            .await?;
        self.refresh_user_cache(user_id, &cart_id).await?;
        self.guest_repo.delete(guest_id).await?;

        tracing::info!(
            user_id,
            guest_id,
            cart_id,
            items = requests.len(),
            "guest cart merged into user cart"
        );
        Ok(())
    }

    async fn refresh_user_cache(&self, user_id: &str, cart_id: &str) -> Result<(), CartError> {
        let items = self.user_repo.load_items(cart_id).await?;
        let cart = CachedCart {
            items,
            updated_at: Utc::now(),
        };
        self.user_repo.put_cached(user_id, &cart).await
    }
}
