"use client";

import { getUser, getUserSchema } from "@/server/queries";
import { TUserContext, UserContext } from "@/state/userContext";
import { PropsWithChildren, useEffect, useState } from "react";

export function UserProvider(props: PropsWithChildren<{}>) {
  const [user, setUser] = useState<TUserContext>({ weakAuth: false });

  useEffect(() => {
    async function checkLoggedIn() {
      const res = await getUser();
      if (res.ok) {
        const user = getUserSchema.transformer(
          getUserSchema.schema.parse(await res.json())
        );
        console.log("LOGGED IN");
        setUser({
          weakAuth: true,
          loggedIn: true,
          user: user.user,
        });
      }
    }

    checkLoggedIn();
  }, []);

  const value = { user, setUser };
  return (
    <UserContext.Provider value={value}>{props.children}</UserContext.Provider>
  );
}
