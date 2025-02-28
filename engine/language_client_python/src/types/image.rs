use pyo3::prelude::{pymethods, PyAnyMethods, PyModule, PyResult};
use pyo3::types::PyType;
use pyo3::{Bound, Py, PyAny, PyObject, Python, ToPyObject};

use crate::errors::BamlInvalidArgumentError;
crate::lang_wrapper!(BamlImagePy, baml_types::BamlMedia);

#[pymethods]
impl BamlImagePy {
    #[staticmethod]
    fn from_url(url: String) -> Self {
        BamlImagePy {
            inner: baml_types::BamlMedia::url(baml_types::BamlMediaType::Image, url, None),
        }
    }

    #[staticmethod]
    fn from_base64(media_type: String, base64: String) -> Self {
        BamlImagePy {
            inner: baml_types::BamlMedia::base64(
                baml_types::BamlMediaType::Image,
                base64,
                Some(media_type),
            ),
        }
    }

    pub fn is_url(&self) -> bool {
        matches!(&self.inner.content, baml_types::BamlMediaContent::Url(_))
    }

    pub fn as_url(&self) -> PyResult<String> {
        match &self.inner.content {
            baml_types::BamlMediaContent::Url(url) => Ok(url.url.clone()),
            _ => Err(BamlInvalidArgumentError::new_err("Image is not a URL")),
        }
    }

    pub fn as_base64(&self) -> PyResult<Vec<String>> {
        match &self.inner.content {
            baml_types::BamlMediaContent::Base64(base64) => Ok(vec![
                base64.base64.clone(),
                self.inner.mime_type.clone().unwrap_or("".to_string()),
            ]),
            _ => Err(BamlInvalidArgumentError::new_err("Image is not base64")),
        }
    }

    pub fn __repr__(&self) -> String {
        match &self.inner.content {
            baml_types::BamlMediaContent::Url(url) => {
                format!("BamlImagePy(url={})", url.url)
            }
            baml_types::BamlMediaContent::Base64(base64) => {
                format!(
                    "BamlImagePy(base64={}, media_type={})",
                    base64.base64,
                    self.inner.mime_type.clone().unwrap_or("".to_string())
                )
            }
            _ => format!("Unknown BamlImagePy variant"),
        }
    }

    // Makes it work with Pydantic
    #[classmethod]
    pub fn __get_pydantic_core_schema__(
        _cls: Bound<'_, PyType>,
        _source_type: Bound<'_, PyAny>,
        _handler: Bound<'_, PyAny>,
    ) -> PyResult<PyObject> {
        Python::with_gil(|py| {
            let code = r#"
from pydantic_core import core_schema

def get_schema():
    # No validation
    return core_schema.any_schema()

ret = get_schema()
    "#;
            // py.run(code, None, Some(ret_dict));
            let fun: Py<PyAny> = PyModule::from_code_bound(py, code, "", "")?
                .getattr("ret")?
                .into();
            Ok(fun.to_object(py)) // Return the PyObject
        })
    }

    pub fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}
