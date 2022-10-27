use serde::{Deserialize, Serialize};

/// Their type receives two generics. The first one in cases of positive
/// responses, and the former in cases with the fetch action could not be
/// performed. During `NotFound` cases, a optional string could be specified,
/// allowing to include a message explaining about the possible reasons of the
/// record not found.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FetchResponse<T, U> {
    Found(T),
    NotFound(U, Option<String>),
}

/// This is like the simple FetchResponse, but includes a vector response
/// instead the simple one.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FetchManyResponse<T, U> {
    Found(Vec<T>),
    NotFound(U, Option<String>),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CreateResponse<T, U> {
    Created(T),
    NotCreated(U, String),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CreateManyResponse<T, U> {
    Created(Vec<T>),
    NotCreated(Vec<U>, String),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GetOrCreateResponse<T, U> {
    Created(T),
    NotCreated(U, Option<String>),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UpdateResponse<T, U> {
    Updated(T),
    NotUpdated(U, Option<String>),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UpdateManyResponse<T, U> {
    Updated(Vec<T>),
    NotUpdated(Vec<U>, Option<String>),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeleteResponse<T> {
    Deleted,
    NotDeleted(T),
}