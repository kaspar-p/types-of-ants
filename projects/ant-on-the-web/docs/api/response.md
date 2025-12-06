# API Response

All of the APIs for `ant-on-the-web` give their responses in similar shapes:

```ts
{
  /**
   * The status boolean of the request.
   * Redundant with statusCode, but used in case statusCode parsing
   * is not needed.
   */
  success: boolean;

  /**
   * The three-digit status code indicating the success of the request.
   * The same status code is duplicated in the header.
   */
  statusCode: number;

  /**
   * A more detailed, human-readable status code. Useful for more
   * information than 404 NOT_FOUND, perhaps a user was NOT_FOUND, or
   * an ant, or another resource.
   */
  statusMessage: string;

  /**
   * The unique request UUID given to this request.
   */
  requestId: string;
  /**
   * The UTC millisecond timestamp that the request was received by
   * the server
   */
  requestReceived: string;

  /**
   * The ID of the host that served the request. Can be used in
   * conjunction with /api/hosts/host/:id to learn more about the host
   * that served the request.
   */
  hostId: string;

  /**
   * The human-readable label for the host
   */
  hostName: string;

  /**
   * The route-specific data that is returned for each request.
   */
  data: any;
}
```
