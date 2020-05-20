const { createProxyMiddleware } = require('http-proxy-middleware');
const Bundler = require('parcel');
const express = require('express');

const bundler = new Bundler('src/index.html', {
  cache: false,
});

const app = express();
const PORT = process.env.PORT || 3000;

app.use(
  '/charts/',
  createProxyMiddleware({
    target: 'http://localhost:3333',
  })
);

app.use(bundler.middleware());

console.log(`Connect on: http://localhost:${PORT}/`,);

app.listen(PORT);