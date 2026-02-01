import { signup } from "@/server/posts";
import { useRouter } from "next/navigation";
import { useState } from "react";

export type SignupBoxProps = {
  setWeakAuth: (weakAuth: boolean) => void;
};

export function SignupBox(props: SignupBoxProps) {
  const [formState, setFormState] = useState<
    { loading: false; success: boolean; msg: string } | { loading: true }
  >({ loading: false, success: false, msg: "" });

  const [username, setUsername] = useState("");
  const [usernameValidationMsg, setUsernameValidationMsg] = useState("");

  const [password, setPassword] = useState("");
  const [password2, setPassword2] = useState("");
  const [passwordValidationMsg, setPasswordValidationMsg] = useState("");

  const { push } = useRouter();

  async function handle(e: any) {
    e.preventDefault();

    setFormState({ loading: true });
    const res = await signup({ username, password, password2 });

    switch (res.__status) {
      default:
      case 500: {
        setFormState({ loading: false, success: false, msg: res.msg });
        break;
      }

      case 409: {
        setFormState({
          loading: false,
          success: false,
          msg: res.msg.toLowerCase(),
        });
        break;
      }

      case 400: {
        for (const error of res.errors) {
          switch (error.field) {
            case "username": {
              setUsernameValidationMsg(error.msg.toLowerCase());
              setFormState({ loading: false, success: false, msg: "" });
              break;
            }
            case "password": {
              setPasswordValidationMsg(error.msg.toLowerCase());
              setFormState({ loading: false, success: false, msg: "" });
              break;
            }
            default:
              return setFormState({
                loading: false,
                success: false,
                msg: "invalid field, please retry.",
              });
          }
        }
        break;
      }

      case 200: {
        setUsername("");
        setUsernameValidationMsg("");
        setPassword("");
        setPasswordValidationMsg("");

        setFormState({
          loading: false,
          success: true,
          msg: "signup complete, welcome!",
        });

        props.setWeakAuth(true);

        push("/login/two-factor");
        break;
      }
    }
  }

  return (
    <div>
      <div className="mb-2">if you don&apos;t have an account, signup:</div>
      <form autoComplete="off" onSubmit={(event) => handle(event)}>
        <div className="grid grid-cols-3 gap-0">
          <span className="flex flex-col justify-center">your username: </span>
          <input
            className="m-1"
            type="text"
            name="username"
            autoComplete="off"
            placeholder="ex. kaspar"
            value={username}
            onChange={(e) => {
              setUsername(e.target.value);
              setUsernameValidationMsg("");
            }}
          />
          <span
            className={`flex flex-col justify-center m-1 text-red-600 content-center`}
          >
            {usernameValidationMsg}
          </span>
          <span className="flex flex-col justify-center">your password: </span>
          <input
            className="m-1"
            type="password"
            autoComplete="off"
            name="password"
            value={password}
            onChange={(e) => {
              setPassword(e.target.value);
              setPasswordValidationMsg("");
            }}
          />
          <span
            className={`flex flex-col justify-center m-1 text-red-600 content-center`}
          >
            {passwordValidationMsg}
          </span>

          <span className="flex flex-col justify-center">
            repeat password:{" "}
          </span>
          <input
            className="m-1"
            type="password"
            autoComplete="off"
            name="password2"
            value={password2}
            onChange={(e) => {
              setPassword2(e.target.value);
              setPasswordValidationMsg("");
            }}
          />
          <span
            className={`flex flex-col justify-center m-1 text-red-600 content-center`}
          >
            {" "}
          </span>
        </div>

        <div className="flex flex-row w-8/12">
          <input type="submit" className="w-full m-1" value="signup" />
        </div>
        <span
          className={`m-1 text-${
            formState.loading ? "blue" : formState.success ? "green" : "red"
          }-600 content-center`}
        >
          {formState.loading ? "loading..." : formState.msg}
        </span>
      </form>
    </div>
  );
}
