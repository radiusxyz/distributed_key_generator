
use async_trait::async_trait;
use dkg_primitives::{AuthError, AuthService, Parameter};
use alloy::{primitives::{Address as EthAddress, U256}, providers::{ProviderBuilder, RootProvider}, sol, transports::http::{reqwest::Url, Client, Http}};

use DkgContract::DkgContractInstance;

sol! {
    #[sol(rpc)]
    contract DkgContract {
        function isAuthority(address account) public view returns (bool);
        function isSolver(address account) public view returns (bool);
        function isCommittee(uint256 round, address account) public view returns (bool);
        function getAuthorityInfo() public view returns (address account, string memory clusterRpcUrl, string memory externalRpcUrl);
        function getSolverInfo() public view returns (address account, string memory clusterRpcUrl, string memory externalRpcUrl);
        function getCommitteeList(uint256 round) public view returns (address[] memory);
        function updateTrustedSetup(bytes memory trusted_setup, bytes memory signature) public;
        function getTrustedSetup() public view returns (bytes memory);
    }
}

#[derive(Clone)]
/// Client that interacts with the blockchain
pub struct DkgAuth {
    pub contract: DkgContractInstance<Http<Client>, RootProvider<Http<Client>>>,
}

impl DkgAuth {
    pub fn new(endpoint: &str, address: &str) -> Self {
        let url = Url::parse(endpoint).unwrap();
        let address = address.parse::<EthAddress>().unwrap();
        let provider = ProviderBuilder::new().on_http(url);
        let contract = DkgContract::new(address, provider);
        Self { contract }
    }
}

fn convert<Address>(address: EthAddress) -> Address 
where
    Address: From<Vec<u8>>
{
    let address = address.0.to_vec();
    address.into()
}

fn convert_back<Address>(address: Address) -> Option<EthAddress> 
where
    Address: AsRef<[u8]>,
{
    let address = address.as_ref().to_vec();
    if address.len() != 20 {
        return None;
    }
    let eth_address = EthAddress::from_slice(&address);
    Some(eth_address)
}

#[async_trait]
impl<Address> AuthService<Address> for DkgAuth 
where
    Address: Parameter + From<Vec<u8>> + AsRef<[u8]>,
{
    type Error = AuthError;

    async fn update_trusted_setup(&self, bytes: Vec<u8>, signature: Vec<u8>) -> Result<(), Self::Error> {
        let _ = self.contract.updateTrustedSetup(bytes.into(), signature.into()).send().await.map_err(|e| AuthError::AnyError(e.to_string()))?;
        Ok(())
    }
    async fn get_trusted_setup(&self) -> Result<Vec<u8>, Self::Error> {
        let bytes = self.contract.getTrustedSetup().call().await.map_err(|e| AuthError::AnyError(e.to_string()))?;
        Ok(bytes._0.to_vec())
    }
    async fn get_authority_info(&self) -> Result<(Address, String, String), Self::Error> {
        let authority_info = self.contract.getAuthorityInfo().call().await.map_err(|e| AuthError::AnyError(e.to_string()))?;
        Ok((convert(authority_info.account), authority_info.clusterRpcUrl, authority_info.externalRpcUrl))
    }
    async fn get_solver_info(&self) -> Result<(Address, String, String), Self::Error> {
        let solver_info = self.contract.getSolverInfo().call().await.map_err(|e| AuthError::AnyError(e.to_string()))?;
        Ok((convert(solver_info.account), solver_info.clusterRpcUrl, solver_info.externalRpcUrl))
    }
    async fn is_active(&self, current_round: u64, address: Address) -> Result<bool, Self::Error> { 
        let is_active = self.contract.isCommittee(U256::from(current_round), convert_back(address).ok_or(AuthError::AnyError("Invalid address".to_string()))?).call().await.map_err(|e| AuthError::AnyError(e.to_string()))?;
        Ok(is_active._0)
    }
    async fn current_auth_registry(&self, current_round: u64) -> Result<Vec<Address>, Self::Error> { 
        let committee_list = self.contract.getCommitteeList(U256::from(current_round)).call().await.map_err(|e| AuthError::AnyError(e.to_string()))?;
        Ok(committee_list._0.into_iter().map(|address| convert(address)).collect())
    }
    async fn next_auth_registry(&self, next_round: u64) -> Result<Vec<Address>, Self::Error> { 
        let committee_list = self.contract.getCommitteeList(U256::from(next_round)).call().await.map_err(|e| AuthError::AnyError(e.to_string()))?;
        Ok(committee_list._0.into_iter().map(|address| convert(address)).collect())
    }
    async fn is_ready(&self, current_round: u64, threshold: u16) -> Result<bool, Self::Error> { 
        let committee_list = self.contract.getCommitteeList(U256::from(current_round)).call().await.map_err(|e| AuthError::AnyError(e.to_string()))?;
        let committee_list_length = committee_list._0.len();
        if committee_list_length < threshold as usize {
            return Ok(false);
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn dkg_auth_works() {
        let dkg_auth = DkgAuth::new("http://localhost:8545", "0x5FbDB2315678afecb367f032d93F642f64180aa3"); 
        let authority_address = "0x70997970C51812dc3A010C7d01b50e0d17dc79C8".parse::<EthAddress>().unwrap();
        let is_authority = dkg_auth.contract.isAuthority(authority_address).call().await.unwrap();
        let solver_address = "0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC".parse::<EthAddress>().unwrap();
        let is_solver = dkg_auth.contract.isAuthority(solver_address).call().await.unwrap();
        println!("{:?}", is_authority._0);
        println!("{:?}", is_solver._0);
        let authority_info = dkg_auth.contract.getAuthorityInfo().call().await.unwrap();
        println!("{:?}", authority_info.account);
        println!("{:?}", authority_info.clusterRpcUrl);
        println!("{:?}", authority_info.externalRpcUrl);
        let solver_info = dkg_auth.contract.getSolverInfo().call().await.unwrap();
        println!("{:?}", solver_info.account);
        println!("{:?}", solver_info.clusterRpcUrl);
        println!("{:?}", solver_info.externalRpcUrl);
    }
}