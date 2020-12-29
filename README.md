https://github.com/11Takanori/actix-web-clean-architecture-sample

## Todos

- auth for controllers and authorization
- serde derive camelcase
- More api tests for [calendarevent, booking]
- better error handling: https://auth0.com/blog/build-an-api-in-rust-with-jwt-authentication-using-actix-web/
- https://developer.makeplans.net/#services


## backlog

- use the different results that are unused
- smarter mongodb schema
- think about how to do auth (nettu ee will likely also use this and maybe nettmeet)
  - should it be just microservice for nettu to start with ? (public / private cert jwt, check google)
  - same as nettu meeting with external api calling endpoints ?
  - both
