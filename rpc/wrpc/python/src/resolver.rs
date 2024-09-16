use crate::client::RpcClient;
use kaspa_consensus_core::network::{NetworkId, NetworkType};
use kaspa_python_macros::py_async;
use kaspa_wrpc_client::{Resolver as NativeResolver, WrpcEncoding};
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use std::{str::FromStr, sync::Arc};

#[derive(Debug, Clone)]
#[pyclass]
pub struct Resolver {
    resolver: NativeResolver,
}

impl Resolver {
    pub fn new(resolver: NativeResolver) -> Self {
        Self { resolver }
    }
}

#[pymethods]
impl Resolver {
    #[new]
    pub fn ctor(urls: Option<Vec<String>>, tls: Option<bool>) -> PyResult<Resolver> {
        let tls = tls.unwrap_or(false);
        if let Some(urls) = urls {
            Ok(Self { resolver: NativeResolver::new(Some(urls.into_iter().map(|url| Arc::new(url)).collect::<Vec<_>>()), tls) })
        } else {
            Ok(Self { resolver: NativeResolver::default() })
        }
    }
}

#[pymethods]
impl Resolver {
    fn urls(&self) -> Vec<String> {
        self.resolver.urls().unwrap_or_default().into_iter().map(|url| (*url).clone()).collect::<Vec<_>>()
    }

    fn get_node(&self, py: Python, encoding: String, network: String, network_suffix: Option<u32>) -> PyResult<Py<PyAny>> {
        let encoding = WrpcEncoding::from_str(encoding.as_str()).unwrap();
        let network_id = into_network_id(&network, network_suffix)?;
        let resolver = self.resolver.clone();

        py_async! {py, async move {
            let node = resolver.get_node(encoding, network_id).await?;
            Python::with_gil(|py| {
                Ok(serde_pyobject::to_pyobject(py, &node).unwrap().to_object(py))
            })
        }}
    }

    fn get_url(&self, py: Python, encoding: String, network: String, network_suffix: Option<u32>) -> PyResult<Py<PyAny>> {
        let encoding = WrpcEncoding::from_str(encoding.as_str()).unwrap();
        let network_id = into_network_id(&network, network_suffix)?;
        let resolver = self.resolver.clone();

        py_async! {py, async move {
            let url = resolver.get_url(encoding, network_id).await?;
            Ok(url)
        }}
    }

    fn connect(&self, py: Python, encoding: String, network: String, network_suffix: Option<u32>) -> PyResult<RpcClient> {
        let encoding = WrpcEncoding::from_str(encoding.as_str()).unwrap();
        let network_id = into_network_id(&network, network_suffix)?;
        let client = RpcClient::new(Some(self.clone()), None, Some(encoding), Some(network_id))?;

        client.connect(py, None, None, None, None, None)?;
        Ok(client)
    }
}

impl From<Resolver> for NativeResolver {
    fn from(resolver: Resolver) -> Self {
        resolver.resolver
    }
}

impl From<NativeResolver> for Resolver {
    fn from(resolver: NativeResolver) -> Self {
        Self { resolver }
    }
}

pub fn into_network_id(network: &str, network_suffix: Option<u32>) -> Result<NetworkId, PyErr> {
    let network_type = NetworkType::from_str(network).map_err(|_| PyErr::new::<PyException, _>("Invalid network type"))?;
    NetworkId::try_from(network_type).or_else(|_| {
        network_suffix.map_or_else(
            || Err(PyErr::new::<PyException, _>("Network suffix required for this network")),
            |suffix| Ok(NetworkId::with_suffix(network_type, suffix)),
        )
    })
}
