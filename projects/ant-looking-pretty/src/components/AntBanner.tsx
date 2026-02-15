import { getLatestAnts } from "../server/queries";
import { ErrorBoundary } from "./UnhappyPath";
import { AntBannerInner } from "./AntBannerInner";
import { Suspense } from "react";

export async function AntBanner() {
  const ants = await getLatestAnts();

  return (
    <ErrorBoundary isError={!ants.success}>
      <AntBannerInner bannerAnts={ants.data} />
    </ErrorBoundary>
  );
}
