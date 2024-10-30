use crate::repo::Store;

#[derive(Clone)]
pub struct AppState<R>
where
    R: Store,
{
    pub repo: R,
}
