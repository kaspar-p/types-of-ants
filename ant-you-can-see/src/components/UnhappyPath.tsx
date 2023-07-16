import React from "react";

export function Error() {
  return <div>error...</div>;
}

export function Loading() {
  return <div>loading...</div>;
}

export function errorOr<
  const NullMap extends Record<string, T>,
  const ReqMap extends { [x in keyof NullMap]: NonNullable<NullMap[x]> },
  const T
>(
  loadingResults: boolean | boolean[],
  errorResults: boolean | boolean[],
  dataResults: NullMap,
  callback: (props: ReqMap) => JSX.Element
): JSX.Element {
  const loading = [loadingResults].flat().some((x) => x);
  const error =
    [errorResults].flat().some((x) => x) ||
    !dataResults ||
    Object.values(dataResults).some((x) => x === undefined);

  if (loading) return <Loading />;
  if (error) return <Error />;
  console.log("dataresults: ", dataResults);
  return callback(dataResults as unknown as ReqMap);
}
