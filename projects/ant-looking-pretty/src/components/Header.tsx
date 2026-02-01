import { getTotalAnts, getVersion } from "@/server/queries";
import {
  dehydrate,
  HydrationBoundary,
  QueryClient,
} from "@tanstack/react-query";
import { Link } from "./Link";
import { Button } from "./Button";
import { LogoutButton } from "./LogoutButton";
import { VersionNumber } from "./VersionNumber";
import { AntsDiscoveredToDate } from "./AntsDiscoveredToDate";
import { getAuth } from "@/state/user";
import { webAction } from "@/server/posts";

export async function Header() {
  const user = await getAuth();

  const queryClient = new QueryClient();

  await queryClient.prefetchQuery({
    queryKey: ["totalAnts"],
    queryFn: () => getTotalAnts(),
  });

  await queryClient.prefetchQuery({
    queryKey: ["version"],
    queryFn: getVersion,
  });

  return (
    <>
      <div className="p-5" style={{ fontFamily: "serif" }}>
        <div className="flex flex-row align-center justify-center">
          <h1 className="mb-0 pb-5">
            types of ants{" "}
            <span className="text-xs font-mono">
              v1.
              <HydrationBoundary state={dehydrate(queryClient)}>
                <VersionNumber />
              </HydrationBoundary>
            </span>
          </h1>
        </div>

        <h3 className="text-center m-0">
          ants discovered to date:{" "}
          <HydrationBoundary state={dehydrate(queryClient)}>
            <AntsDiscoveredToDate />
          </HydrationBoundary>
        </h3>

        <div className="flex flex-col gap-y-2 py-4 max-w-md mx-auto">
          <div className="text-center flex flex-row gap-x-2 align-center justify-center">
            {!user.loggedIn && <Button path="/login">log in / signup</Button>}
            <Button path="/">home</Button>
            {user.loggedIn && (
              <>
                <Button path="/profile">profile</Button>
                <Button path="/feed">feed</Button>
              </>
            )}

            {user.loggedIn && <LogoutButton>logout</LogoutButton>}
          </div>

          <div className="text-center flex flex-row gap-x-2 align-center justify-center">
            <Link href="https://twitter.com/typesofants">
              <button className="cursor-pointer">twitter @typesofants</button>
            </Link>
            <Link href="https://github.com/kaspar-p/types-of-ants">
              <button id="github-link" className="cursor-pointer">
                ant who wants to read the code
              </button>
            </Link>
          </div>
        </div>
      </div>
    </>
  );
}
