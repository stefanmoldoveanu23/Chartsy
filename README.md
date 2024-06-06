## Plans
- Stylize pages(replace all images with icons/widgets, go back buttons to all scenes, other details maybe later)
- Fix similarities trigger to take into consideration account deletion and reinstatement
- Handle all cases of accessing deleted user data(comments)
- App icon
- As time constraints may allow
    - restructure into services
    - take into consideration loss of connection
    - more tooling functionality
    - translate+scale drawing
    - add reference images(+ text notes)
    - image rendering optimizations?
    - consider dropbox data deletion upon automatic account deletion
    - TBA


Cache fix:
- have both an async and a sync cache;
- the async cache holds longer TTL data and gets it from the database;
- the sync cache holds shorter TTL data and gets it from the async cache;
- for the scenes that need caching, at every update go through all needed images and:
    - check if they exist in the sync cache;
    - get it from the async cache with try_get_with(this only calls the function if it doesn't exist)
    - insert into sync cache.