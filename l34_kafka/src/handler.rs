use crate::{
    error::Error,
    model::{Id, Product, User},
    repo::Store,
    state::AppState,
};
use axum::{
    extract::{Path, State},
    Json,
};
use validator::Validate;

pub async fn create_user<R>(
    State(state): State<AppState<R>>,
    Json(user): Json<User>,
) -> Result<(), Error>
where
    R: Store,
{
    user.validate()?;
    state.repo.create_user(user).await
}

pub async fn update_user<R>(
    State(state): State<AppState<R>>,
    Path(ref user_id): Path<Id>,
    Json(user): Json<User>,
) -> Result<(), Error>
where
    R: Store,
{
    user.validate()?;
    state.repo.update_user(user_id, user).await
}

pub async fn delete_user<R>(
    State(state): State<AppState<R>>,
    Path(ref user_id): Path<Id>,
) -> Result<(), Error>
where
    R: Store,
{
    state.repo.delete_user(user_id).await
}

pub async fn create_product<R>(
    State(state): State<AppState<R>>,
    Json(product): Json<Product>,
) -> Result<(), Error>
where
    R: Store,
{
    state.repo.create_product(product).await
}

pub async fn update_product<R>(
    State(state): State<AppState<R>>,
    Path(ref product_id): Path<Id>,
    Json(product): Json<Product>,
) -> Result<(), Error>
where
    R: Store,
{
    state.repo.update_product(product_id, product).await
}

pub async fn delete_product<R>(
    State(state): State<AppState<R>>,
    Path(ref product_id): Path<Id>,
) -> Result<(), Error>
where
    R: Store,
{
    state.repo.delete_product(product_id).await
}
