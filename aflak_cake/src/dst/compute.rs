use rayon;
use variant_name::VariantName;

use dst::{DST, DSTError, Output, OutputId};

impl<'t, T: 't, E: 't> DST<'t, T, E>
where
    T: Clone + VariantName + Send + Sync,
    E: Send,
{
    fn _compute(&self, output: Output) -> Result<T, DSTError<E>> {
        let t = self.get_transform(&output.t_idx).ok_or_else(|| {
            DSTError::ComputeError(format!("Tranform {:?} not found!", output.t_idx))
        })?;
        let output_cache_lock = self.cache.get(&output).expect("Get output cache");
        {
            let output_cache = output_cache_lock.read().unwrap();
            if let Some(ref cache) = *output_cache {
                return Ok(cache.clone());
            }
        }
        let deps = self.get_transform_dependencies(&output.t_idx);
        let mut op = t.start();
        let mut results = Vec::with_capacity(deps.len());
        for _ in 0..(deps.len()) {
            results.push(Err(DSTError::NothingDoneYet));
        }
        rayon::scope(|s| {
            for (result, parent_output) in results.iter_mut().zip(deps) {
                s.spawn(move |_| {
                    *result = parent_output
                        .ok_or_else(|| {
                            DSTError::ComputeError("Missing dependency! Cannot compute.".to_owned())
                        })
                        .and_then(|output| self._compute(output));
                })
            }
        });
        for result in results {
            op.feed(result?);
        }
        match op.call().nth(output.output_i.into()) {
            None => Err(DSTError::ComputeError(
                "No nth output received. This is a bug!".to_owned(),
            )),
            Some(result) => {
                if let Ok(ref result) = result {
                    let mut cache = output_cache_lock.write().unwrap();
                    *cache = Some(result.clone())
                }
                result.map_err(|err| DSTError::InnerComputeError(err))
            }
        }
    }

    /// Return the result of the computation to the output given as argument.
    ///
    /// If possible, computation is distributed on several threads.
    pub fn compute(&self, output_id: &OutputId) -> Result<T, DSTError<E>> {
        self.outputs
            .get(output_id)
            .ok_or_else(|| {
                DSTError::MissingOutputID(format!("Output ID {:?} not found!", output_id))
            })
            .and_then(|output| {
                output.ok_or_else(|| {
                    DSTError::MissingOutputID(format!("Output ID {:?} is not attached!", output_id))
                })
            })
            .and_then(|output| self._compute(output))
    }
}
