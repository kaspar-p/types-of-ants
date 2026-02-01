"use client";

import { logout } from "@/server/posts";
import { PropsWithChildren } from "react";
import { useUser } from "@/app/UserProvider";

export const LogoutButton = (props: PropsWithChildren<{}>) => {
  const { resetUser } = useUser();

  const handleLogout = async () => {
    const res = await logout();
    console.log(res);
    switch (res.__status) {
      case 200: {
        break;
      }
      default: {
        console.log("wrong");
      }
    }
    resetUser();
  };

  return (
    <button className="cursor-pointer" onClick={() => handleLogout()}>
      logout
    </button>
  );
};
