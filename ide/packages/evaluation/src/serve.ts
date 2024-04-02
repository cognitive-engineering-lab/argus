import fs from "fs";
import http from "http";
import url from "url";

export const PORT = 8080;

export function fileServer() {
  return http.createServer((request, response) => {
    response.setHeader("Access-Control-Allow-Origin", "*");
    response.setHeader("Access-Control-Request-Method", "*");
    response.setHeader("Access-Control-Allow-Methods", "OPTIONS, GET");
    response.setHeader("Access-Control-Allow-Headers", "*");

    const reject = () => {
      response.writeHead(404, { "Content-Type": "text/plain" });
      response.write("404 Not Found\n");
      response.end();
    };

    if (request.url === undefined) {
      console.error("No URL provided");
      return reject();
    }

    const filename = url.pathToFileURL(request.url).pathname;
    if (!fs.existsSync(filename)) {
      console.error("File does not exist", filename);
      return reject();
    }

    const file = fs.readFileSync(filename, "binary");
    response.writeHead(200);
    response.write(file, "binary");
    response.end();
  });
}
