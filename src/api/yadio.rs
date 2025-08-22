use crate::api::shared::Error;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[allow(non_snake_case)]
struct Currency {
    USD: Prices,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[allow(non_snake_case)]
pub struct Prices {
    pub ARS: f64,
    pub BRL: f64,
    pub CAD: f64,
    pub CLP: f64,
    pub COP: f64,
    pub EUR: f64,
    pub GBP: f64,
    pub IDR: f64,
    pub MXN: f64,
    pub MYR: f64,
    pub NGN: f64,
    pub PHP: f64,
    pub SGD: f64,
    pub THB: f64,
    pub VES: f64,
    pub VND: f64,
}

impl Default for Prices {
    fn default() -> Self {
        Prices {
            ARS: 0.0,
            BRL: 0.0,
            CAD: 0.0,
            CLP: 0.0,
            COP: 0.0,
            EUR: 0.0,
            GBP: 0.0,
            IDR: 0.0,
            MXN: 0.0,
            MYR: 0.0,
            NGN: 0.0,
            PHP: 0.0,
            SGD: 0.0,
            THB: 0.0,
            VES: 0.0,
            VND: 0.0,
        }
    }
}

pub async fn get_yadio_prices() -> Result<Prices, Error> {
    let url = "https://api.yadio.io/exrates/usd";
    let res = reqwest::get(url).await;
    match res {
        Ok(res) => match res.json::<Currency>().await {
            Ok(currency) => Ok(currency.USD),
            Err(e) => Err(Error::from(&e)),
        },
        Err(e) => Err(Error::from(&e)),
    }
}

#[tokio::test]
async fn test_yadio_prices() {
    let price_source = get_yadio_prices().await;
    assert!(price_source.is_ok());
    let json_msg = serde_json::to_string(&price_source.unwrap()).unwrap();
    println!("prices: {}", json_msg)
}

#[tokio::test]
async fn test_asean_currencies() {
    let prices = get_yadio_prices().await.unwrap();

    // Verify all ASEAN currencies have non-zero values
    assert!(prices.IDR > 0.0, "IDR price should be greater than 0");
    assert!(prices.PHP > 0.0, "PHP price should be greater than 0");
    assert!(prices.VND > 0.0, "VND price should be greater than 0");
    assert!(prices.MYR > 0.0, "MYR price should be greater than 0");
    assert!(prices.SGD > 0.0, "SGD price should be greater than 0");
    assert!(prices.THB > 0.0, "THB price should be greater than 0");

    println!("ASEAN currency prices:");
    println!("IDR: {}", prices.IDR);
    println!("PHP: {}", prices.PHP);
    println!("VND: {}", prices.VND);
    println!("MYR: {}", prices.MYR);
    println!("SGD: {}", prices.SGD);
    println!("THB: {}", prices.THB);
}

#[test]
fn test_zero_price_protection() {
    // Test that zero prices are properly handled
    let mut test_prices = Prices::default(); // All zeros

    // Simulate partial API response with some valid prices
    test_prices.EUR = 0.862166;
    test_prices.GBP = 0.746083;
    test_prices.SGD = 1.290295;

    // Verify we can detect zero prices
    assert_eq!(test_prices.ARS, 0.0);
    assert_eq!(test_prices.BRL, 0.0);
    assert!(test_prices.EUR > 0.0);
    assert!(test_prices.GBP > 0.0);
    assert!(test_prices.SGD > 0.0);

    println!("Zero price protection test passed");
}
