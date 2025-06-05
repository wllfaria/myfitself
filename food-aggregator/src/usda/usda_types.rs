use serde::Deserialize;

use crate::supervisor::{FoodData, FoodEntry, FoodEntryNutrient};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsdaFoodSearchResponse {
    pub total_pages: usize,
    pub foods: Vec<UsdaFoodSearchFood>,
}

impl FoodData for UsdaFoodSearchResponse {
    type Entry = UsdaFoodSearchFood;
    type EntryIter<'a> = std::slice::Iter<'a, Self::Entry>;

    fn entries(&self) -> Self::EntryIter<'_> {
        self.foods.iter()
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsdaFoodSearchFood {
    pub fdc_id: i32,
    pub description: String,
    #[serde(default)]
    pub food_code: Option<i32>,
    #[serde(default)]
    pub food_category: Option<String>,
    #[serde(default)]
    pub food_category_id: Option<i32>,
    pub food_nutrients: Vec<UsdaFoodNutrient>,
}

impl FoodEntry for UsdaFoodSearchFood {
    type Nutrient = UsdaFoodNutrient;
    type NutrientIter<'a> = std::slice::Iter<'a, Self::Nutrient>;

    fn source(&self) -> String {
        String::from("USDA")
    }

    fn wweia_data(&self) -> Option<(i32, &String)> {
        self.food_category_id
            .map(|id| (id, self.food_category.as_ref().unwrap()))
    }

    fn name(&self) -> &str {
        &self.description
    }

    fn fndds_code(&self) -> Option<i32> {
        self.food_code
    }

    fn id(&self) -> i32 {
        self.fdc_id
    }

    fn nutrients(&self) -> Self::NutrientIter<'_> {
        self.food_nutrients.iter()
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsdaFoodNutrient {
    pub nutrient_name: String,
    pub unit_name: String,
    #[serde(default)]
    pub value: Option<f32>,
}

impl FoodEntryNutrient for UsdaFoodNutrient {
    fn name(&self) -> &str {
        &self.nutrient_name
    }

    fn unit_name(&self) -> &str {
        &self.unit_name
    }

    fn value(&self) -> f32 {
        self.value.unwrap_or_default()
    }
}
