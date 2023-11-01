from fastapi import FastAPI
from fastapi.responses import HTMLResponse, FileResponse
from fastapi.staticfiles import StaticFiles

app = FastAPI()
app.mount("/static", StaticFiles(directory="static"), name="static")


@app.get("/favicon.png")
async def favicon():
    statics = StaticFiles(directory="static")
    file = await statics.get_response("favicon.png", {"method": "GET", "headers": {}})
    return file

@app.get("/")
def read_root():
    return {"Hello": "World"}


@app.get("/{year}/leaderboard/day/{day}", response_class=HTMLResponse)
def global_leaderboard(year: int, day: int):
    with open(f"./global/{year}_{day:02}.html") as f:
        html = f.read()
    return html

@app.get("/{year}/leaderboard/private/view/{id}.json", response_class=FileResponse)
def private_leaderboard(year: int, id: int):
    return f"./private/{year}_{id}.json"

@app.get("/{year}/day/{day}", response_class=HTMLResponse)
def daily_challenge(year: int, day: int):
    with open(f"./challenges/{year}_{day:02}.html") as f:
        html = f.read()
    return html
