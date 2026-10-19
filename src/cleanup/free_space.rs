use std::path::Path;

use super::registry::Registry;
use super::{ByteLimit, Cleanup, Invocation, Workspace};
use crate::error::{CargoOperation, Error, FileOperation};

pub(super) struct FreeSpacePolicy {
    limit: ByteLimit,
    registry: Registry,
}

impl FreeSpacePolicy {
    pub(super) fn new(limit: ByteLimit) -> Self {
        Self {
            limit,
            registry: Registry::default(),
        }
    }

    pub(super) fn apply(
        &mut self,
        invocation: &Invocation<'_>,
        workspace: &Workspace,
        target: &Path,
    ) -> Result<(), Error> {
        self.registry = Registry::register(&workspace.workspace_root, target)?;
        let available = fs2::available_space(&workspace.workspace_root).map_err(|source| {
            Error::file(FileOperation::FreeSpace, &workspace.workspace_root, source)
        })?;
        if available < self.limit.get()
            && let Some(project) = self.registry.heaviest()?
        {
            let command = if project.is_current(&workspace.workspace_root, target)? {
                invocation.command(CargoOperation::Clean)
            } else {
                project.clean_command()
            };
            Cleanup::clean(command, project.target())?;
        }
        Ok(())
    }
}
