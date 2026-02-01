import { ErrorBoundary } from "./UnhappyPath";
import { getVersion } from "@/server/queries";

export const VersionNumber = async () => {
  const version = await getVersion();

  return (
    <ErrorBoundary isError={!version.success} fallback="0000">
      {version.data}
    </ErrorBoundary>
  );
};
