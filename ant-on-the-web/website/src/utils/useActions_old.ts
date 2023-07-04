// import { suggestAnt } from "@/server/posts";
// import { FormEvent } from "react";

// export function useActions() {
//   type Action = {
//     containerID: string;
//     replaceChildID: string;
//     inputID: string;
//     validator: (text: string) => Validation;
//     endpoint: string;
//     handling: keyof typeof actions;
//   };

//   type Validation = {
//     valid: boolean;
//     msg: string;
//   };

//   const develop = false;
//   const isHandling = {
//     newAnt: false,
//     newsletter: false,
//   };

//   async function handle(event: FormEvent<HTMLFormElement>, action: Action) {
//     event.preventDefault();

//     const {
//       containerID,
//       replaceChildID,
//       inputID,
//       validator,
//       endpoint,
//       handling,
//     } = action;

//     const container = document.getElementById(containerID);
//     if (!container) throw new Error("No element with id: " + containerID);
//     const toReplace = document.getElementById(replaceChildID);
//     if (!toReplace) throw new Error("No element with id: " + replaceChildID);

//     const responseText = document.createElement("div");
//     responseText.classList.add("replacer");

//     container.replaceChild(responseText, toReplace);

//     const input = document.getElementById(inputID) as HTMLInputElement;
//     if (!input) throw new Error("No element with id: " + inputID);
//     input.value = input.value.trim();
//     if (isHandling[handling]) {
//       return;
//     }

//     const { valid, msg } = validator(input.value);
//     if (!valid) {
//       responseText.style.color = "red";
//       responseText.innerText = msg;
//     } else {
//       let counter = 0;
//       const dotInterval = setInterval(() => {
//         responseText.style.color = "blue";
//         responseText.innerText = "loading" + ".".repeat(counter % 5);
//         counter++;
//       }, 100);

//       // Send the request
//       const url = "http://localhost:3499";

//       await fetch(`${url}/${endpoint}`, {
//         method: "POST",
//         body: input.value,
//       })
//         .then((response) => {
//           clearInterval(dotInterval);
//           if (response.ok && response.status === 200) {
//           }
//           return response.json();
//         })
//         .then((json) => {
//           const { status, msg, userExists } = json;
//           if (status === 200 && userExists) {
//             responseText.style.color = "red";
//             responseText.innerText = "you're already subscribed!";
//           } else if (status === 200) {
//             responseText.style.color = "green";
//             responseText.innerText = "thanks!";
//           } else {
//             throw new Error(json);
//           }
//         })
//         .catch((error) => {
//           clearInterval(dotInterval);
//           responseText.style.color = "red";
//           responseText.innerText = "error encountered, input not processed!";
//         });
//     }

//     // Make text appear and clear input
//     isHandling[handling] = true;
//     input.value = "";
//     setTimeout(() => {
//       container.replaceChild(toReplace, responseText);
//       isHandling[handling] = false;
//     }, 3000);
//   }

//   function newAntIsValid(text: string): Validation {
//     let msg = "";
//     if (text.length <= 2) {
//       msg = "ant too short!";
//     } else if (text.length >= 100) {
//       msg = "ant too long!";
//     }

//     return {
//       valid: msg === "",
//       msg,
//     };
//   }

//   function newsletterIsValid(text: string): Validation {
//     let msg = "";
//     if (
//       !/(?:[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*|"(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21\x23-\x5b\x5d-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])*")@(?:(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?|\[(?:(?:(2(5[0-5]|[0-4][0-9])|1[0-9][0-9]|[1-9]?[0-9]))\.){3}(?:(2(5[0-5]|[0-4][0-9])|1[0-9][0-9]|[1-9]?[0-9])|[a-z0-9-]*[a-z0-9]:(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21-\x5a\x53-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])+)\])/.test(
//         text
//       )
//     ) {
//       msg = "invalid email!";
//     }

//     return {
//       valid: msg === "",
//       msg,
//     };
//   }

//   const actions = {
//     newAnt: {
//       containerID: "new-ant-form-container",
//       replaceChildID: "new-ant-replacer",
//       request: suggestAnt,
//       inputID: "new-ant",
//       validator: newAntIsValid,
//       handling: "newAnt",
//       endpoint: "api/new-ant",
//     },
//     newsletter: {
//       containerID: "newsletter-form-container",
//       replaceChildID: "newsletter-replacer",
//       request: suggestAnt,
//       inputID: "newsletter",
//       validator: newsletterIsValid,
//       handling: "newsletter",
//       endpoint: "api/ant-newsletter",
//     },
//   } as const;

//   return { handle, actions };
// }
