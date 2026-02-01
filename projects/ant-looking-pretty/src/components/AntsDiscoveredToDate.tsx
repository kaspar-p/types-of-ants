import { ErrorBoundary } from "./UnhappyPath";
import { getTotalAnts } from "@/server/queries";

export async function AntsDiscoveredToDate() {
  const totalAnts = await getTotalAnts();

  return (
    <ErrorBoundary isError={!totalAnts.success} fallback={"..."}>
      {totalAnts.data}
    </ErrorBoundary>
  );
}
