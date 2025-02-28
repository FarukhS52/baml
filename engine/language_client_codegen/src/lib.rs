use anyhow::{Context, Result};
use indexmap::IndexMap;
use internal_baml_core::{
    configuration::{GeneratorDefaultClientMode, GeneratorOutputType},
    ir::repr::IntermediateRepr,
};
use std::{collections::BTreeMap, path::PathBuf};
use version_check::{check_version, GeneratorType, VersionCheckMode};

mod dir_writer;
mod openapi;
mod python;
mod ruby;
mod typescript;
pub mod version_check;

pub struct GeneratorArgs {
    /// Output directory for the generated client, relative to baml_src
    output_dir_relative_to_baml_src: PathBuf,

    /// Path to the BAML source directory
    baml_src_dir: PathBuf,

    inlined_file_map: BTreeMap<PathBuf, String>,

    version: String,
    no_version_check: bool,

    // Default call mode for functions
    default_client_mode: GeneratorDefaultClientMode,
    on_generate: Vec<String>,
}

fn relative_path_to_baml_src(path: &PathBuf, baml_src: &PathBuf) -> Result<PathBuf> {
    pathdiff::diff_paths(path, baml_src).ok_or_else(|| {
        anyhow::anyhow!(
            "Failed to compute relative path from {} to {}",
            path.display(),
            baml_src.display()
        )
    })
}

impl GeneratorArgs {
    pub fn new<'i>(
        output_dir_relative_to_baml_src: impl Into<PathBuf>,
        baml_src_dir: impl Into<PathBuf>,
        input_files: impl IntoIterator<Item = (&'i PathBuf, &'i String)>,
        version: String,
        no_version_check: bool,
        default_client_mode: GeneratorDefaultClientMode,
        on_generate: Vec<String>,
    ) -> Result<Self> {
        let baml_src = baml_src_dir.into();
        let input_file_map: BTreeMap<PathBuf, String> = input_files
            .into_iter()
            .map(|(k, v)| Ok((relative_path_to_baml_src(k, &baml_src)?, v.clone())))
            .collect::<Result<_>>()?;

        Ok(Self {
            output_dir_relative_to_baml_src: output_dir_relative_to_baml_src.into(),
            baml_src_dir: baml_src.clone(),
            // for the key, whhich is the name, just get the filename
            inlined_file_map: input_file_map,
            version,
            no_version_check,
            default_client_mode,
            on_generate,
        })
    }

    pub fn file_map(&self) -> Result<Vec<(String, String)>> {
        self.inlined_file_map
            .iter()
            .map(|(k, v)| {
                Ok((
                    serde_json::to_string(&k.display().to_string()).map_err(|e| {
                        anyhow::Error::from(e)
                            .context(format!("Failed to serialize key {:#}", k.display()))
                    })?,
                    serde_json::to_string(v).map_err(|e| {
                        anyhow::Error::from(e)
                            .context(format!("Failed to serialize contents of {:#}", k.display()))
                    })?,
                ))
            })
            .collect()
    }

    pub fn output_dir(&self) -> PathBuf {
        use sugar_path::SugarPath;
        self.baml_src_dir
            .join(&self.output_dir_relative_to_baml_src)
            .normalize()
    }

    /// Returns baml_src relative to the output_dir.
    ///
    /// We need this to be able to codegen a singleton client, so that our generated code can build
    /// baml_src relative to the path of the file in which the singleton is defined. In Python this is
    /// os.path.dirname(__file__) for globals.py; in TS this is __dirname for globals.ts.
    pub fn baml_src_relative_to_output_dir(&self) -> Result<PathBuf> {
        pathdiff::diff_paths(&self.baml_src_dir, &self.output_dir()).ok_or_else(|| {
            anyhow::anyhow!(
                "Failed to compute baml_src ({}) relative to output_dir ({})",
                self.baml_src_dir.display(),
                self.output_dir().display()
            )
        })
    }
}

pub struct GenerateOutput {
    pub client_type: GeneratorOutputType,
    /// Relative path to the output directory (output_dir in the generator)
    pub output_dir_shorthand: PathBuf,
    /// The absolute path that the generated baml client was written to
    pub output_dir_full: PathBuf,
    pub files: IndexMap<PathBuf, String>,
}

pub trait GenerateClient {
    fn generate_client(&self, ir: &IntermediateRepr, gen: &GeneratorArgs)
        -> Result<GenerateOutput>;
}

// Assume VSCode is the only one that uses WASM, and it does call this method but at a different time.
#[cfg(target_arch = "wasm32")]
fn version_check_with_error(
    runtime_version: &str,
    gen_version: &str,
    generator_type: GeneratorType,
    mode: VersionCheckMode,
    client_type: GeneratorOutputType,
) -> Result<()> {
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn version_check_with_error(
    runtime_version: &str,
    gen_version: &str,
    generator_type: GeneratorType,
    mode: VersionCheckMode,
    client_type: GeneratorOutputType,
) -> Result<()> {
    let res = check_version(
        gen_version,
        runtime_version,
        generator_type,
        mode,
        client_type,
        true,
    );
    match res {
        Some(e) => Err(anyhow::anyhow!("{}", e.msg())),
        None => Ok(()),
    }
}

impl GenerateClient for GeneratorOutputType {
    fn generate_client(
        &self,
        ir: &IntermediateRepr,
        gen: &GeneratorArgs,
    ) -> Result<GenerateOutput> {
        let runtime_version = env!("CARGO_PKG_VERSION");

        if !gen.no_version_check {
            version_check_with_error(
                runtime_version,
                &gen.version,
                GeneratorType::CLI,
                VersionCheckMode::Strict,
                self.clone(),
            )?;
        }

        let files = match self {
            GeneratorOutputType::OpenApi => openapi::generate(ir, gen),
            GeneratorOutputType::PythonPydantic => python::generate(ir, gen),
            GeneratorOutputType::RubySorbet => ruby::generate(ir, gen),
            GeneratorOutputType::Typescript => typescript::generate(ir, gen),
        }?;

        #[cfg(not(target_arch = "wasm32"))]
        {
            for cmd in gen.on_generate.iter() {
                log::info!("Running {:?} in {}", cmd, gen.output_dir().display());
                let status = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(cmd)
                    .current_dir(gen.output_dir())
                    .status()
                    .context(format!("Failed to run on_generate command {:?}", cmd))?;
                if !status.success() {
                    anyhow::bail!(
                        "on_generate command finished with {}: {:?}",
                        match status.code() {
                            Some(code) => format!("exit code {}", code),
                            None => "no exit code".to_string(),
                        },
                        cmd,
                    );
                }
            }

            if matches!(self, GeneratorOutputType::OpenApi) && gen.on_generate.is_empty() {
                // TODO: we should auto-suggest a command for the user to run here
                log::warn!("No on_generate commands were provided for OpenAPI generator - skipping OpenAPI client generation");
            }
        }

        Ok(GenerateOutput {
            client_type: self.clone(),
            output_dir_shorthand: gen.output_dir_relative_to_baml_src.clone(),
            output_dir_full: gen.output_dir(),
            files,
        })
    }
}
