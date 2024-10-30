pub mod postgres;

use crate::error::Error;
use crate::model::{Id, Product, User};
use serde::de::DeserializeOwned;

pub trait ListenEvent {
    async fn listen<T>(
        &self,
        event: impl AsRef<str>,
        handle: impl Fn(T) + Send + 'static,
    ) -> Result<&Self, Error>
    where
        T: DeserializeOwned;
}

pub trait Store: Clone {
    async fn create_user(&self, user: User) -> Result<(), Error>;
    async fn update_user(&self, user_id: &Id, user: User) -> Result<(), Error>;
    async fn delete_user(&self, user_id: &Id) -> Result<(), Error>;

    async fn create_product(&self, product: Product) -> Result<(), Error>;
    async fn update_product(&self, product_id: &Id, product: Product) -> Result<(), Error>;
    async fn delete_product(&self, product_id: &Id) -> Result<(), Error>;
}
