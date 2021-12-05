# riddly

A rust-actix backend tiddlywiki application which can backup to gist.


# deploy to heroku


````bash
heroku git:remote -a your_app_name
heroku config:set WIKI_USER_NAME=test WIKI_USER_PASSWORD=test GITHUB_GIST_TOKEN=test GITHUB_GIST_ID=test
heroku buildpacks:set emk/rust -a your_app_name
git push heroku master
````
