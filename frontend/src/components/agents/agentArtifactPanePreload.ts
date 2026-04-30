let artifactPanePromise: Promise<typeof import("./AgentsArtifactPane")> | null = null;

export function preloadAgentsArtifactPane() {
  artifactPanePromise ??= import("./AgentsArtifactPane").catch((error) => {
    artifactPanePromise = null;
    throw error;
  });
  return artifactPanePromise;
}
