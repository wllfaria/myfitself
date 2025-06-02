use super::usda_types::UsdaFoodSearchResponse;
use crate::FoodSource;

pub struct UsdaClient {
    page_size: usize,
    total_pages: usize,
    api_url: String,
    api_key: String,
}

impl UsdaClient {
    pub fn new() -> Self {
        let api_key = dotenvy::var("USDA_API_KEY").expect("USDA_API_KEY env var must be set");

        let api_url = dotenvy::var("USDA_API_URL").expect("USDA_API_URL env var must be set");
        let api_url = format!("{api_url}/foods/search");

        Self {
            page_size: 200,
            total_pages: 2,
            api_url,
            api_key,
        }
    }
}

impl FoodSource for UsdaClient {
    type Data = UsdaFoodSearchResponse;

    fn is_finished(&self, current_page: usize) -> bool {
        current_page == self.total_pages
    }

    fn fetch(&self, current_page: usize) -> impl Future<Output = anyhow::Result<Self::Data>> {
        Box::pin(async move {
            let client = reqwest::Client::new();
            let request = client
                .get(&self.api_url)
                .query(&[
                    ("api_key", &self.api_key),
                    ("pageSize", &self.page_size.to_string()),
                    ("pageNumber", &current_page.to_string()),
                ])
                .build()
                .expect("malformed USDA search request");

            let response = match client.execute(request).await {
                Ok(res) => res,
                // TODO: if a request fails, we should store it somewhere and try again later
                Err(_) => todo!(),
            };

            // TODO: if a request fails, we should store it somewhere and try again later
            if !response.status().is_success() {
                todo!();
            }

            let data = match response.json::<UsdaFoodSearchResponse>().await {
                Ok(body) => body,
                Err(e) => todo!("{e:?}"),
            };

            // self.total_pages = data.total_pages;

            Ok(data)
        })
    }
}
