const express = require("express")
const app = express()
const bodyParser = require("body-parser");
const jwt = require('jsonwebtoken')
const CronJob = require("cron").CronJob;



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

const windowSlide = new CronJob('0 */2 * * * *', function() {
    console.log(">>>>>>>>>>>>>>>>>>>>>>>>> 2 mins")
	cacheableRules.foo_used = 0
    cacheableRules.bar_used = 0
    nonCacheableRules.baz_used = 0
});

windowSlide.start()

app.post("/auth", (req,res)=>{
    console.log("auth request received by the management service")
    try {
        let decoded = jwt.verify(req.body.token.split(" ")[1],"SECRET")
        console.log("jwt token authentication successful")
        console.log(nonCacheableRules.baz_used, nonCacheableRules.baz_quota)
        if(nonCacheableRules.baz_used == nonCacheableRules.baz_quota){
            res.status(429).send({
                "status": 429,
                "message":"Service quota reached"
            })
        }else{
            nonCacheableRules.baz_used += 1
            res.status(200).send({
                "status": 200,
                "message": "Authenticated",
                "x-rate-limit-header": 20
            })
        }
        
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

    // If no change between local cache and global rules
    if(cacheableRules.foo_used != cacheUpdate.foo_used){
        if(cacheableRules.foo_quota <= cacheUpdate.foo_used){
            console.log("/foo global limit reached")
            cacheableRules.foo_used = cacheableRules.foo_quota
        }else{
            console.log("/foo global limit updated")
            cacheableRules.foo_used = cacheUpdate.foo_used
        }
    }else{
        console.log("/foo local cache no changes")
    }

    // If no change between local cache and global rules
    if(cacheableRules.bar_used != cacheUpdate.bar_used){
        if(cacheableRules.bar_quota <= cacheUpdate.bar_used){
            cacheableRules.bar_used = cacheableRules.bar_quota
            console.log("/bar global limit reached")
        }else{
            cacheableRules.bar_used = cacheUpdate.bar_used
            console.log("/bar global limit updated")
        }
    }else{
         console.log("/bar local cache no changes")
    }
    console.log("cacheableRules updated successfully")
    res.status(200).send(cacheableRules)
})

app.get("/token", (req,res) => {
    console.log("token request received by the management service")
    res.send(jwt.sign("auth-token","SECRET"))
})

app.listen(8000, ()=>{"Management service listening on port 8000"})