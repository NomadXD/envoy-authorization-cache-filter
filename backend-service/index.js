const express = require("express")
const app = express()

app.get("/foo", (req,res)=>{
    console.log("request received by foo service")
    res.status(200).send("Hello from foo service !!!")
})

app.get("/bar", (req,res)=>{
    console.log("request received by bar service")
    res.status(200).send("Hello from bar service !!!")
})

app.get("/baz", (req,res)=>{
    console.log("request received by baz service")
    res.status(200).send("Hello from baz service !!!")
})

app.listen(8000,()=>{"Backend service started listening on port 8000"})