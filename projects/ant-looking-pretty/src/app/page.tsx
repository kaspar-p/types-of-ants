import { AntBanner } from "../components/AntBanner";
import { getReleasedAnts } from "../server/queries";
import { ErrorBoundary } from "@/components/UnhappyPath";
import { InputBanner } from "@/components/InputBanner";
import { AntColumns } from "@/components/AntColumns";

export default async function Home() {
  const releasedAnts = [];
  let nextPage: number | undefined = 0;
  let isError: boolean = false;
  do {
    const res = await getReleasedAnts(nextPage);
    isError = !res.success;

    if (res.success) {
      const data = await res.data;
      if (data.hasNextPage) {
        nextPage += 1;
      } else {
        nextPage = undefined;
      }
      releasedAnts.push(...(data.ants ?? []));
    } else {
      break;
    }
  } while (nextPage !== undefined);

  return (
    <ErrorBoundary isError={isError}>
      <InputBanner />
      <AntBanner />
      <AntColumns ants={releasedAnts} />
    </ErrorBoundary>
  );
}
