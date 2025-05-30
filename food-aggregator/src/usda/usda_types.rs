use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsdaFoodSearchResponse {
    pub total_hits: usize,
    pub current_page: usize,
    pub total_pages: usize,
    pub page_list: Vec<usize>,
    pub food_search_criteria: UsdaFoodSearchCriteria,
    pub foods: Vec<UsdaFoodSearchFood>,
    pub aggregations: UsdaFoodSearchAggregations,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsdaFoodSearchCriteria {
    pub page_number: usize,
    pub number_of_results_per_page: usize,
    pub page_size: usize,
    pub require_all_words: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsdaFoodSearchFood {
    pub fdc_id: usize,
    pub description: String,
    #[serde(default)]
    pub common_names: Option<String>,
    #[serde(default)]
    pub additional_descriptions: Option<String>,
    pub data_type: String,
    #[serde(default)]
    pub food_code: Option<usize>,
    pub published_date: String,
    #[serde(default)]
    pub food_category: Option<String>,
    #[serde(default)]
    pub food_category_id: Option<usize>,
    pub all_highlight_fields: String,
    pub score: f32,
    pub food_nutrients: Vec<UsdaFoodNutrient>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsdaFoodNutrient {
    pub nutrient_id: usize,
    pub nutrient_name: String,
    pub nutrient_number: String,
    pub unit_name: String,
    #[serde(default)]
    pub value: Option<f32>,
    pub rank: usize,
    pub indent_level: usize,
    pub food_nutrient_id: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsdaFoodSearchAggregations {
    pub data_type: UsdaFoodSearchDataType,
}

#[derive(Debug, Deserialize)]
pub struct UsdaFoodSearchDataType {
    #[serde(rename = "Branded")]
    pub branded: usize,
    #[serde(rename = "SR Legacy")]
    pub sr_legacy: usize,
    #[serde(rename = "Survey (FNDDS)")]
    pub fndds_survey: usize,
    #[serde(rename = "Foundation")]
    pub foundation: usize,
    #[serde(rename = "Experimental")]
    pub experimental: usize,
}
