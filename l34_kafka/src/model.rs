use serde::{Deserialize, Serialize};
use validator::Validate;

pub type Id = i32;
pub type Money = i32;

#[allow(unused)]
#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct User {
    #[serde(skip_serializing)]
    pub id: Option<Id>,
    pub name: String,
    #[validate(email)]
    pub email: String,
}

#[allow(unused)]
#[derive(Debug, Deserialize, Serialize)]
pub struct Product {
    #[serde(skip_serializing)]
    pub id: Option<Id>,
    pub name: String,
    pub price: Money,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all(deserialize = "UPPERCASE"))]
pub enum OperationType {
    Insert,
    Update,
    Delete,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UserDiff {
    operation_type: OperationType,
    user_id: Id,
    name_before: Option<String>,
    name_after: Option<String>,
    email_before: Option<String>,
    email_after: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProductDiff {
    operation_type: OperationType,
    product_id: Id,
    name_before: Option<String>,
    name_after: Option<String>,
    price_before: Option<Money>,
    price_after: Option<Money>,
}

#[derive(Debug, Deserialize)]
pub enum BusMessage {
    User(UserDiff),
    Product(ProductDiff),
}
