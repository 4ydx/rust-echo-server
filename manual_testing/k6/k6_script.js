import http from "k6/http";
// import { sleep } from "k6";

export const options = {
  vus: 10,
  duration: "30s",
};

export default function() {
  // http.get("http://localhost:9999/");

  let time = new Date().getTime();
  let headers = { "Content-Type": "application/json" };
  let resp = http.post(
    "http://localhost:9999/endpoint",
    '{ "time": "' +
    time +
    '", "resource_type": "organization", "resource_id": "1" }',
    { headers }
  );
  // console.log(resp.body);
  // sleep(0.001);
}
