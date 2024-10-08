import fs from "node:fs";
import http from "node:http";
import url from "node:url";

export const PORT = 8080;

export async function withServerOnPort<T>(
  port: number,
  callback: () => Promise<T>
) {
  const serve = fileServer();
  try {
    serve.listen(port);
    return await callback();
  } finally {
    serve.closeAllConnections();
    serve.close();
  }
}

function fileServer() {
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
