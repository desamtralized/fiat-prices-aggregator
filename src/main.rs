#[macro_use]
extern crate dotenv_codegen;

pub mod api;
pub mod fiat;
pub mod shared;

use crate::api::yadio::{get_yadio_prices, Prices};
use bip39::Mnemonic;
use cosmrs::{
    bip32,
    cosmwasm::MsgExecuteContract,
    crypto::secp256k1,
    rpc::HttpClient,
    tendermint::chain::Id,
    tx::{self, Fee, Msg, SignDoc, SignerInfo},
    AccountId, Coin,
};
use cosmwasm_std::Uint128;
use dotenv::dotenv;
use localmoney_protocol::{
    currencies::FiatCurrency,
    price::{CurrencyPrice, ExecuteMsg::UpdatePrices},
};
use shared::AccountResponse;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let yadio_request = get_yadio_prices().await;
    let price = match yadio_request {
        Ok(prices) => prices,
        Err(_) => Prices::default(),
    };

    let all_prices = vec![
        (price.ARS, FiatCurrency::ARS),
        (price.BRL, FiatCurrency::BRL),
        (price.CAD, FiatCurrency::CAD),
        (price.CLP, FiatCurrency::CLP),
        (price.COP, FiatCurrency::COP),
        (price.EUR, FiatCurrency::EUR),
        (price.GBP, FiatCurrency::GBP),
        (price.IDR, FiatCurrency::IDR),
        (price.MXN, FiatCurrency::MXN),
        (price.MYR, FiatCurrency::MYR),
        (price.NGN, FiatCurrency::NGN),
        (price.PHP, FiatCurrency::PHP),
        (price.SGD, FiatCurrency::SGD),
        (price.THB, FiatCurrency::THB),
        (price.VES, FiatCurrency::VES),
        (price.VND, FiatCurrency::VND),
    ];

    // Filter out zero prices to prevent posting invalid data
    let valid_prices: Vec<(f64, FiatCurrency)> = all_prices
        .into_iter()
        .filter(|(price, currency)| {
            if *price <= 0.0 {
                println!("WARNING: Skipping {} due to zero/invalid price", currency);
                false
            } else {
                true
            }
        })
        .collect();

    // Check if we have any valid prices to post
    if valid_prices.is_empty() {
        println!("ERROR: No valid prices available. Aborting transaction to prevent posting invalid data.");
        std::process::exit(1);
    }

    println!("Valid prices to post: {}/{}", valid_prices.len(), 16);
    let prices_json = serde_json::to_string(&valid_prices).unwrap();
    println!("prices: {}", prices_json);
    // Derivate Wallet from Seed
    let path = "m/44'/118'/0'/0/0"
        .parse::<bip32::DerivationPath>()
        .unwrap();
    let seed_words = dotenv!("ADMIN_SEED");
    let mnemonic = Mnemonic::parse_normalized(&seed_words).unwrap();
    let seed = mnemonic.to_seed("");
    let sender_priv_key = secp256k1::SigningKey::derive_from_path(seed, &path).unwrap();
    let sender_pub_key = sender_priv_key.public_key();
    let sender_addr = sender_pub_key.account_id(dotenv!("ADDR_PREFIX")).unwrap();

    // Fetch Account details, we need the account sequence number
    let account_url = format!(
        "{}cosmos/auth/v1beta1/accounts/{}",
        dotenv!("LCD"),
        sender_addr.to_string()
    );
    let account_res = reqwest::get(account_url).await.unwrap();
    let account_data = account_res.json::<AccountResponse>().await;
    println!("account_data ok: {}", account_data.is_ok());
    let account_data = account_data.unwrap();
    println!("Account sequence is {}", account_data.account.sequence);

    // Send Tx to Contract
    let contract_addr = dotenv!("PRICE_ADDR").parse::<AccountId>().unwrap();
    let mut tx_body_builder = tx::BodyBuilder::new();
    let mut currency_prices: Vec<CurrencyPrice> = vec![];
    valid_prices.iter().for_each(|price_fiat| {
        // TODO: although most currencies have 2 decimals,
        // some currencies like JPY have 3 and some exotic currencies have zero.
        let usd_price = Uint128::from((price_fiat.0 * 100.0).round() as u64);
        let fiat_currency = price_fiat.1.clone();
        currency_prices.push(CurrencyPrice {
            currency: fiat_currency,
            usd_price,
            updated_at: 0,
        });
    });
    // let update_price_msg = UpdatePrices()
    let update_prices_msg = UpdatePrices(currency_prices);
    let json_msg = serde_json::to_string(&update_prices_msg).unwrap();
    println!("update_msg: {}", json_msg);
    let execute_msg = MsgExecuteContract {
        sender: sender_addr.clone(),
        contract: contract_addr.clone(),
        msg: json_msg.into_bytes(),
        funds: vec![],
    };
    tx_body_builder.msg(execute_msg.into_any().unwrap());

    let signer_info = SignerInfo::single_direct(
        Some(sender_pub_key.clone()),
        account_data
            .account
            .sequence
            .clone()
            .parse::<i64>()
            .unwrap() as u64,
    );
    let gas_amount = Coin {
        amount: 15000u128,
        denom: "uatom".parse().unwrap(),
    };
    let auth_info = signer_info.auth_info(Fee::from_amount_and_gas(gas_amount, 500_000u64));
    let tx_body = tx_body_builder.finish();
    let account_number = account_data
        .account
        .account_number
        .clone()
        .parse::<i64>()
        .unwrap() as u64;
    let chain_id = dotenv!("CHAIN_ID").parse::<Id>().unwrap();
    let sign_doc = SignDoc::new(&tx_body, &auth_info, &chain_id, account_number).unwrap();
    let tx_signed = sign_doc.sign(&sender_priv_key).unwrap();
    let rpc_url = dotenv!("RPC");
    let client = HttpClient::new(rpc_url).unwrap();
    let res = tx_signed.broadcast_commit(&client).await.unwrap();
    println!("{}", res.tx_result.log.to_string());
    println!("res: {:#?}", res);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::yadio::Prices;

    #[test]
    fn test_price_filtering_logic() {
        // Create test prices with some zeros
        let mut test_price = Prices::default();
        test_price.EUR = 0.85;
        test_price.GBP = 0.75;
        test_price.SGD = 1.30;
        // ARS, BRL, CAD, etc. remain 0.0

        let all_prices = vec![
            (test_price.ARS, FiatCurrency::ARS), // 0.0
            (test_price.BRL, FiatCurrency::BRL), // 0.0
            (test_price.EUR, FiatCurrency::EUR), // 0.85
            (test_price.GBP, FiatCurrency::GBP), // 0.75
            (test_price.SGD, FiatCurrency::SGD), // 1.30
        ];

        // Filter out zero prices
        let valid_prices: Vec<(f64, FiatCurrency)> = all_prices
            .into_iter()
            .filter(|(price, _currency)| *price > 0.0)
            .collect();

        // Should only have 3 valid prices
        assert_eq!(valid_prices.len(), 3);
        assert!(valid_prices
            .iter()
            .any(|(_, c)| matches!(c, FiatCurrency::EUR)));
        assert!(valid_prices
            .iter()
            .any(|(_, c)| matches!(c, FiatCurrency::GBP)));
        assert!(valid_prices
            .iter()
            .any(|(_, c)| matches!(c, FiatCurrency::SGD)));

        // Should not have zero-priced currencies
        assert!(!valid_prices
            .iter()
            .any(|(_, c)| matches!(c, FiatCurrency::ARS)));
        assert!(!valid_prices
            .iter()
            .any(|(_, c)| matches!(c, FiatCurrency::BRL)));
    }
}
