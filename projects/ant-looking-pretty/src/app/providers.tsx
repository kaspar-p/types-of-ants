"use client";

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { PropsWithChildren, useEffect, useState } from "react";
import { TUserContext, UserProvider } from "@/app/UserProvider";
import { getAuth } from "@/state/user";

export const Providers = (
  props: PropsWithChildren<{
    user: Promise<TUserContext>;
    resetUser: () => void;
  }>,
) => {
  const queryClient = new QueryClient();

  return (
    <QueryClientProvider client={queryClient}>
      <UserProvider user={props.user} resetUser={props.resetUser}>
        {props.children}
      </UserProvider>
    </QueryClientProvider>
  );
};
