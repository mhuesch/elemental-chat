use hdk::prelude::*;
use crate::message::Message;
/// This is the current structure of the payload the holo signs
#[hdk_entry(id = "joining_code_payload")]
#[derive(Clone)]
struct JoiningCodePayload {
    role: String,
    record_locator: String
}

/// Validate joining code from the membrane_proof
pub(crate) fn joining_code(membrane_proof: Option<MembraneProof>) -> ExternResult<ValidateCallbackResult> {
    // This is a hard coded holo agent public key
    let holo_agent = AgentPubKey::try_from("uhCAkfzycXcycd-OS6HQHvhTgeDVjlkFdE2-XHz-f_AC_5xelQX1N").unwrap();
    match membrane_proof {
        Some(mem_proof) => {
            let mem_proof = match Element::try_from(mem_proof.clone()) {
                Ok(m) => m,
                Err(e) => return Ok(ValidateCallbackResult::Invalid(format!("Joining code invalid: unable to deserialize into element ({:?})", e)))
            };

            debug!("Joining code provided: {:?}", mem_proof);

            let author = mem_proof.header().author().clone();

            if author != holo_agent {
                debug!("Joining code validation failed");
                return Ok(ValidateCallbackResult::Invalid(format!("Joining code invalid: unexpected author ({:?})", author)))
            }

            if let ElementEntry::Present(_entry) = mem_proof.entry() {
                let signature = mem_proof.signature().clone();
                if verify_signature(holo_agent.clone(), signature, mem_proof.header())? {
                    debug!("Joining code validated");
                    return Ok(ValidateCallbackResult::Valid)
                } else {
                    debug!("Joining code validation failed: incorrect signature");
                    return Ok(ValidateCallbackResult::Invalid("Joining code invalid: incorrect signature".to_string()))
                }
            } else {
                return Ok(ValidateCallbackResult::Invalid("Joining code invalid payload".to_string()));
            }

        }
        None => Ok(ValidateCallbackResult::Invalid("No membrane proof found".to_string()))
    }
}

pub(crate) fn common_validatation(data: ValidateData) -> ExternResult<ValidateCallbackResult> {
    let element = data.element.clone();
    let entry = element.into_inner().1;
    let entry = match entry {
        ElementEntry::Present(e) => e,
        _ => return Ok(ValidateCallbackResult::Valid),
    };
    if let Entry::Agent(_) = entry {
        match data.element.header().prev_header() {
            Some(header) => {
                match get(header.clone(), GetOptions::default()) {
                    Ok(element_pkg) => match element_pkg {
                        Some(element_pkg) => {
                            match element_pkg.signed_header().header() {
                                Header::AgentValidationPkg(pkg) => {
                                    return joining_code(pkg.membrane_proof.clone())
                                }
                                _ => return Ok(ValidateCallbackResult::Invalid("No Agent Validation Pkg found".to_string()))
                            }
                        },
                        None => return Ok(ValidateCallbackResult::UnresolvedDependencies(vec![(header.clone()).into()]))
                    },
                    Err(e) => {
                        debug!("Error on get when validating agent entry: {:?}; treating as unresolved dependency",e);
                        return Ok(ValidateCallbackResult::UnresolvedDependencies(vec![(header.clone()).into()]))
                    }
                }
            },
            None => return Ok(ValidateCallbackResult::Invalid("Impossible state".to_string()))
        }
    }
    Ok(match Message::try_from(&entry) {
        Ok(message) => {
            if message.content.len() <= 1024 {
                ValidateCallbackResult::Valid
            } else {
                ValidateCallbackResult::Invalid("Message too long".to_string())
            }
        }
        _ => ValidateCallbackResult::Valid,
    })
}
