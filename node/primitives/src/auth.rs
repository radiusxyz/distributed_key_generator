use std::str::FromStr;
use async_trait::async_trait;
use dkg_primitives::{AuthServiceError, AuthService, Parameter, KeyGenerator, Round};
use alloy::{
    network::{Ethereum, EthereumWallet}, primitives::{Address as EthAddress, U256}, providers::{fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller}, Identity, ProviderBuilder, RootProvider}, signers::local::LocalSigner, sol, sol_types::SolValue, transports::http::{reqwest::Url, Client, Http}
};
use crate::key_service::SkdeParams;

type ContractInstance = DkgContract::DkgContractInstance<
    Http<Client>,
    FillProvider<
        JoinFill<
            JoinFill<
                Identity,
                JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
            >,
            WalletFiller<EthereumWallet>,
        >,
        RootProvider<Http<Client>>,
        Http<Client>,
        Ethereum,
    >,
>;

sol! {
    #[sol(rpc)]
    contract DkgContract {
        struct CommitteeInfo {
            address account;
            string clusterRpcUrl;
            string externalRpcUrl;
        }

        struct TrustedSetupParams {
            string n;
            string g;
            uint32 t;
            string h;
            string max_sequencer_number;
        }

        function isAuthority(address account) public view returns (bool);
        function isSolver(address account) public view returns (bool);
        function isCommittee(uint256 round, address account) public view returns (bool);
        function getAuthorityInfo() public view returns (address account, string memory clusterRpcUrl, string memory externalRpcUrl);
        function getSolverInfo() public view returns (address account, string memory clusterRpcUrl, string memory externalRpcUrl);
        function getCommitteeList(uint256 round) public view returns (CommitteeInfo[] memory);
        function updateTrustedSetup(bytes memory params, bytes memory authoritySignature) public;
        function getTrustedSetup() public view returns (bytes memory);
        function registerCommittee(uint256 round, address account, string memory clusterRpcUrl, string memory externalRpcUrl) public;
        function unregisterCommittee(uint256 round, address account) public;    
    }
}

impl From<DkgContract::TrustedSetupParams> for SkdeParams {
    fn from(params: DkgContract::TrustedSetupParams) -> Self {
        Self {
            n: params.n,
            g: params.g,
            t: params.t,
            h: params.h,
            max_sequencer_number: params.max_sequencer_number,
        }
    }
}

#[derive(Clone)]
/// Client that interacts with the blockchain
pub struct DefaultAuthService {
    pub contract: ContractInstance,
}

impl DefaultAuthService {
    pub fn new(endpoint: &str, private_key: &str, contract_address: &str) -> Self {
        let url = Url::parse(endpoint).unwrap();
        let signer = LocalSigner::from_str(private_key).unwrap();
        let wallet = EthereumWallet::new(signer);
        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet)
            .on_http(url);
        let contract = DkgContract::new(contract_address.parse::<EthAddress>().unwrap(), provider);
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
impl<Address> AuthService<Address> for DefaultAuthService 
where
    Address: Parameter + From<Vec<u8>> + AsRef<[u8]>
{
    type TrustedSetup = DkgContract::TrustedSetupParams;
    type Error = AuthServiceError;

    async fn update_trusted_setup<T>(&self, trusted_setup: T, signature: Vec<u8>) -> Result<(), Self::Error> 
    where
        T: Into<Self::TrustedSetup> + Send + Sync + 'static
    {
        let trusted_setup: Self::TrustedSetup = trusted_setup.into();
        let bytes = trusted_setup.abi_encode();
        let _ = self.contract
            .updateTrustedSetup(bytes.into(), signature.into())
            .gas(15_000_000)
            .gas_price(20000000000)
            .send()
            .await
            .map_err(|e| AuthServiceError::AnyError(e.to_string()))?;
        Ok(())
    }
    async fn get_trusted_setup<T>(&self) -> Result<T, Self::Error>
    where
        Self::TrustedSetup: Into<T>
    {
        let res = self.contract.getTrustedSetup().call().await.map_err(|e| AuthServiceError::AnyError(e.to_string()))?;
        let trusted_setup = Self::TrustedSetup::abi_decode(&res._0.to_vec(), false).map_err(|e| AuthServiceError::AnyError(e.to_string()))?;
        Ok(trusted_setup.into())
    }
    async fn get_solver_info(&self) -> Result<(Address, String, String), Self::Error> {
        let res = self.contract.getSolverInfo().call().await.map_err(|e| AuthServiceError::AnyError(e.to_string()))?;
        Ok((convert(res.account), res.clusterRpcUrl, res.externalRpcUrl))
    }
    async fn register_key_generator(&self, round: Round, address: Address, cluster_rpc_url: &str, external_rpc_url: &str) -> Result<(), Self::Error> {
        let _ = self.contract.registerCommittee(U256::from(round.0), convert_back(address).ok_or(AuthServiceError::AnyError("Invalid address".to_string()))?, cluster_rpc_url.to_string(), external_rpc_url.to_string()).call().await.map_err(|e| AuthServiceError::AnyError(e.to_string()))?;
        Ok(())
    }
    async fn unregister_key_generator(&self, round: Round, address: Address) -> Result<(), Self::Error> {
        let _ = self.contract.unregisterCommittee(U256::from(round.0), convert_back(address).ok_or(AuthServiceError::AnyError("Invalid address".to_string()))?).call().await.map_err(|e| AuthServiceError::AnyError(e.to_string()))?;
        Ok(())
    }
    async fn is_active(&self, current_round: Round, address: Address) -> Result<bool, Self::Error> { 
        let res = self.contract.isCommittee(U256::from(current_round.0), convert_back(address).ok_or(AuthServiceError::AnyError("Invalid address".to_string()))?).call().await.map_err(|e| AuthServiceError::AnyError(e.to_string()))?;
        Ok(res._0)
    }
    async fn get_key_generators(&self, current_round: &Round) -> Result<Vec<KeyGenerator<Address>>, Self::Error> { 
        let res = self.contract.getCommitteeList(U256::from(current_round.0)).call().await.map_err(|e| AuthServiceError::AnyError(e.to_string()))?;
        Ok(res._0.into_iter().map(|info| KeyGenerator::new(convert(info.account), info.clusterRpcUrl, info.externalRpcUrl)).collect())
    }
    async fn is_ready(&self, current_round: &Round, threshold: u16) -> Result<bool, Self::Error> {
        let res: Vec<KeyGenerator<Address>> = self.get_key_generators(current_round).await?;
        Ok(res.len() >= threshold as usize)
    }
}
