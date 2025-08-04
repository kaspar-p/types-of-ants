"use client";

import { changeUsername } from "@/server/posts";
import { getUser, getUserSchema } from "@/server/queries";
import { UserContext } from "@/state/userContext";
import { FormEvent, useContext, useState } from "react";

type ChangeUsernameBoxProps = {
  onSuccess: () => void | Promise<void>;
};

export default function ChangeUsernameBox(props: ChangeUsernameBoxProps) {
  const { user, setUser } = useContext(UserContext);
  const [username, setUsername] = useState<string>("");
  const [usernameValidationMsg, setUsernameValidationMsg] = useState<{
    valid: boolean;
    msg: string;
  }>({ valid: false, msg: "" });

  async function handleNewUsername(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();

    console.log("NEW: ", username);

    const usernameRes = await changeUsername({ username });
    switch (usernameRes.status) {
      case 500: {
        setUsernameValidationMsg({
          valid: false,
          msg: "something went wrong, please retry",
        });

        break;
      }

      case 409: {
        setUsernameValidationMsg({
          valid: false,
          msg: "username already taken!",
        });
        break;
      }

      case 400: {
        const e: { errors: { field: string; msg: string }[] } =
          await usernameRes.json();

        setUsernameValidationMsg({
          valid: false,
          msg: e.errors[0].msg.toLocaleLowerCase(),
        });

        break;
      }

      case 200: {
        setUsernameValidationMsg({ valid: true, msg: "username changed!" });
        setUsername("");

        const userRes = await getUser();
        if (!userRes.ok) return;
        const user = getUserSchema.transformer(
          getUserSchema.schema.parse(await userRes.json())
        );
        setUser({ weakAuth: true, loggedIn: true, user: user.user });

        await props.onSuccess();
      }
    }
  }

  return (
    <>
      <h2>enter new username</h2>
      <div>change the username of your account</div>
      <form autoComplete="off" onSubmit={(e) => handleNewUsername(e)}>
        <div className="grid grid-cols-3 gap-0">
          <span className="flex flex-col justify-center">username:</span>
          <input
            className="m-1"
            type="text"
            name="username"
            autoComplete="off"
            placeholder=""
            value={username}
            onChange={(e) => {
              setUsername(e.target.value);
              setUsernameValidationMsg({ valid: false, msg: "" });
            }}
          />
          <span
            className={`flex flex-col justify-center m-1 text-red-600 content-center`}
          >
            {" "}
          </span>
        </div>
        <div className="w-8/12">
          <input type="submit" value="change username" />
          <span
            className={`flex flex-col justify-center m-1 text-${
              usernameValidationMsg.valid ? "green" : "red"
            }-600 content-center`}
          >
            {usernameValidationMsg.msg}
          </span>
        </div>
      </form>
    </>
  );
}
