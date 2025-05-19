import { signup } from "@/server/posts";
import { useState } from "react";

export function SignupBox() {
  const [formState, setFormState] = useState<
    { loading: false; success: boolean; msg: string } | { loading: true }
  >({ loading: false, success: false, msg: "" });

  const [username, setUsername] = useState("");
  const [usernameValidationMsg, setUsernameValidationMsg] = useState("");

  const [phone, setPhone] = useState("");
  const [phoneValidationMsg, setPhoneValidationMsg] = useState("");

  const [email, setEmail] = useState("");
  const [emailValidationMsg, setEmailValidationMsg] = useState("");

  const [password, setPassword] = useState("");
  const [passwordValidationMsg, setPasswordValidationMsg] = useState("");

  async function handle(e: any) {
    e.preventDefault();

    setFormState({ loading: true });
    const response = await signup({
      username: username,
      phoneNumber: phone,
      email: email,
      password: password,
    });

    switch (response.status) {
      default:
      case 500: {
        return setFormState({
          loading: false,
          success: false,
          msg: "something went wrong, please retry.",
        });
      }

      case 409: {
        const j: { msg: string } = await response.json();
        return setFormState({
          loading: false,
          success: false,
          msg: j.msg.toLowerCase(),
        });
      }

      case 400: {
        const j: { field: string; msg: string } = await response.json();
        switch (j.field) {
          case "phoneNumber": {
            setPhoneValidationMsg(j.msg.toLowerCase());
            setFormState({ loading: false, success: false, msg: "" });
            break;
          }
          case "email": {
            setEmailValidationMsg(j.msg.toLowerCase());
            setFormState({ loading: false, success: false, msg: "" });
            break;
          }
          case "username": {
            setUsernameValidationMsg(j.msg.toLowerCase());
            setFormState({ loading: false, success: false, msg: "" });
            break;
          }
          case "password": {
            setPasswordValidationMsg(j.msg.toLowerCase());
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

      case 200: {
        const j = await response.text();

        setPhone("");
        setPhoneValidationMsg("");
        setUsername("");
        setUsernameValidationMsg("");
        setEmail("");
        setEmailValidationMsg("");
        setPassword("");
        setPasswordValidationMsg("");

        setFormState({
          loading: false,
          success: true,
          msg: "signup complete, welcome!",
        });
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
            onChange={(e) => setUsername(e.target.value)}
          />
          <span
            className={`flex flex-col justify-center m-1 text-red-600 content-center`}
          >
            {usernameValidationMsg}
          </span>

          <span className="flex flex-col justify-center">
            your phone number:{" "}
          </span>
          <input
            className="m-1"
            type="text"
            name="phoneNumber"
            autoComplete="off"
            placeholder="ex. +1 (000) 111-2222"
            value={phone}
            onChange={(e) => setPhone(e.target.value)}
          />
          <span
            className={`flex flex-col justify-center m-1 text-red-600 content-center`}
          >
            {phoneValidationMsg}
          </span>

          <span className="flex flex-col justify-center">your email: </span>
          <input
            className="m-1"
            type="text"
            name="email"
            autoComplete="off"
            placeholder="ex. kaspar@typesofants.org"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
          />
          <span
            className={`flex flex-col justify-center m-1 text-red-600 content-center`}
          >
            {emailValidationMsg}
          </span>

          <span className="flex flex-col justify-center">your password: </span>
          <input
            className="m-1"
            type="password"
            autoComplete="off"
            name="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
          <span
            className={`flex flex-col justify-center m-1 text-red-600 content-center`}
          >
            {passwordValidationMsg}
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
