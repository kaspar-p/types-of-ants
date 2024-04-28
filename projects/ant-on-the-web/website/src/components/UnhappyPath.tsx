import React, { ReactNode } from "react";

export function ErrorBoundary(props: {
  isError: boolean;
  children: ReactNode;
}) {
  if (props.isError) return <div>error...</div>;
  return props.children;
}

export function LoadingBoundary(props: {
  isLoading: boolean;
  children: ReactNode;
}) {
  if (props.isLoading) return <div>loading...</div>;
  return props.children;
}

// export function errorOr<
//   const NullMap extends Record<string, T>,
//   const ReqMap extends { [x in keyof NullMap]: NonNullable<NullMap[x]> },
//   const T
// >(
//   loadingResults: boolean | boolean[],
//   errorResults: boolean | boolean[],
//   dataResults: NullMap,
//   callback: (props: ReqMap) => JSX.Element
// ): JSX.Element {
//   const loading = [loadingResults].flat().some((x) => x);
//   const error =
//     [errorResults].flat().some((x) => x) ||
//     !dataResults ||
//     Object.values(dataResults).some((x) => x === undefined);

//   if (loading) return <Loading />;
//   if (error) return <Error />;
//   console.log("dataresults: ", dataResults);
//   return callback(dataResults as unknown as ReqMap);
// }
