import http from "k6/http";
// import { sleep } from "k6";

export const options = {
  vus: 10,
  duration: "30s",
};

export default function () {
  // http.get("http://localhost:9999/");

  let headers = { "Content-Type": "application/json" };
  http.post(
    "http://localhost:9999/endpoint",
    '{ "user_id": "2", "resource_type": "organization", "resource_id": "1" }',
    { headers }
  );

  // sleep(0.001);
}
