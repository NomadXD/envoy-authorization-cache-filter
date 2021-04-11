const express = require("express")
const app = express()
const bodyParser = require("body-parser");
const jwt = require('jsonwebtoken')

app.use(bodyParser.json());

global.cacheableRules = {
    "foo_path":  "/foo",
    "foo_quota": 10,
    "foo_used":  0,
    "bar_path":  "/bar",
    "bar_quota": 10,
    "bar_used":  0
}

global.nonCacheableRules = {
    "baz_path": "/baz",
    "baz_quota": 10,
    "baz_used": 0
}

app.post("/auth", (req,res)=>{
    console.log("auth request received by the management service")
    try {
        let decoded = jwt.verify(req.body.token,"SECRET")
        console.log("jwt token authentication successful")
        res.status(200).send({
            "status": 200,
            "message": "Authenticated",
            "x-rate-limit-header": 20
        })
    } catch (error) {
        console.log("jwt token authentication failed")
        res.status(401).send({
            "status": 401,
            "message": "Unauthenticated",
            "x-rate-limit-header": 20
        })
    }
})

app.get("/cache", (req,res)=>{
    console.log("initial cache update request received")
    res.status(200).send(cacheableRules)
})

app.post("/cache", (req, res) => {
    console.log("periodical cache udapte request received")
    console.log(req.body)
    cacheUpdate = req.body

    if(cacheableRules.foo_quota <= cacheableRules.foo_used + cacheUpdate.foo_used){
        cacheableRules.foo_used = cacheableRules.foo_quota
    }else{
        cacheableRules.foo_used += cacheUpdate.foo_used 
    }

    if(cacheableRules.bar_quota <= cacheableRules.bar_used + cacheUpdate.bar_used){
        cacheableRules.bar_used = cacheableRules.bar_quota
    }else{
        cacheableRules.bar_used += cacheUpdate.bar_used 
    }
    console.log("cacheableRules updated successfully")
    res.status(200).send(cacheableRules)
})

app.get("/token", (req,res) => {
    console.log("token request received by the management service")
    res.send(jwt.sign("auth-token","SECRET"))
})

app.listen(8000, ()=>{"Management service listening on port 8000"})