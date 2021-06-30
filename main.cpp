#include "raylib.h"
#include <vector>
#include <random>
#include <stdio.h>
#include <cmath>

#include "Entity.hpp"
#include "Player.hpp"

Player initPlayer(int, int);
Entity initBall(int, int);
void ballBounce(Entity* , char);
void drawBricks(std::vector<Entity>&);
void createBricks(std::vector<Entity>&);



int main() {

// Initialization
    //--------------------------------------------------------------------------------------
    
    const int screenWidth = 1280;
    const int screenHeight = 720;
    SetConfigFlags(FLAG_WINDOW_HIGHDPI);
    InitWindow(screenWidth, screenHeight, "BreakOut");

    SetTargetFPS(60);               // Set our game to run at 60 frames-per-second
    std::string actualFPS;
    //--------------------------------------------------------------------------------------
    int lifes = 3;
    int score = 0;
    
    std::string livesLeft;
    std::string scorePrintable;

    Entity ball = initBall(screenWidth, screenHeight);
    Player player = initPlayer(screenWidth, screenHeight);
    
    //Create Some bricks

    std::vector<Entity> bricks;
    createBricks(bricks);
    

    
    // Borders
    
    Rectangle top = {0, 0, screenWidth, 1};
    Rectangle bottom = {0, screenHeight - 1, screenWidth, 1};
    Rectangle left = {0, 0, 1, screenHeight };
    Rectangle right = {screenWidth - 1, 0, 1, screenHeight};
    
    // Main game loop
    while (!WindowShouldClose()) {   // Detect window close button or ESC key
    
        // Update
        //----------------------------------------------------------------------------------
        // TODO: Update your variables here
        //----------------------------------------------------------------------------------
        player.checkInput();
        player.update();
        ball.update();

        if (CheckCollisionRecs(ball.getRectangle(), bottom)) {
            lifes -= 1;
            ball = initBall(screenWidth, screenHeight);
        } else if (CheckCollisionRecs(ball.getRectangle(), top)) {
            ballBounce(&ball, 't');
        } else if (CheckCollisionRecs(ball.getRectangle(), left)) {
            ballBounce(&ball, 'l');
        } else if (CheckCollisionRecs(ball.getRectangle(), right)) {
            ballBounce(&ball, 'r');
        } else if (CheckCollisionRecs(ball.getRectangle(), player.getRectangle())) {
            ballBounce(&ball, 'u');
        }

        if (CheckCollisionRecs(player.getRectangle(), left)) {
            player.preventLeft();
        } else if (player.checkCollision(right)) {
            player.preventRight();
        }
        // Check collision between bricks
        for (int i = 0; i < bricks.size(); i++) {
            if (CheckCollisionRecs(ball.getRectangle(), bricks[i].getRectangle())) {
                // check if below
                if (ball.position.y > bricks[i].position.y + bricks[i].rectangle.height / 2) {
                    ballBounce(&ball, 't');
                }
                // check if above
                if (ball.position.y < bricks[i].position.y - bricks[i].rectangle.height / 2) {
                    ballBounce(&ball, 'u');
                }
                // check if left
                if (ball.position.x < bricks[i].position.x - bricks[i].rectangle.width / 2) {
                    ballBounce(&ball, 'l');
                }
                // chick if right
                if (ball.position.x > bricks[i].position.x + bricks[i].rectangle.width / 2) {
                    ballBounce(&ball, 'r');
                }
                // delete brick
                if (bricks.size() > 1) {
                    // bricks.erase(bricks[i]);
                    bricks.erase(bricks.begin() + i);
                }
                score++;

                break;
            }
        }

        actualFPS = "FPS: " + std::to_string(GetFPS());
        livesLeft = "Lives: " + std::to_string(lifes);
        // Draw
        //----------------------------------------------------------------------------------
        BeginDrawing();

        // DrawRectangleRec(top, RAYWHITE);
        // DrawRectangleRec(bottom, RAYWHITE);
        // DrawRectangleRec(left, RAYWHITE);
        // DrawRectangleRec(right, RAYWHITE);
        ClearBackground(BLACK);
        drawBricks(bricks);

        if (lifes > 0) {
            ball.draw();
            player.draw();
        } else {
            std::string game_over = "Game Over";
            livesLeft = "Lives: 0";
            DrawText(game_over.c_str(), screenWidth / 2 - 25, screenHeight / 2, 40, LIGHTGRAY);
        }
        scorePrintable = "Score: " + std::to_string(score);

        DrawText(actualFPS.c_str(), 25, 25, 20, LIGHTGRAY);
        DrawText(livesLeft.c_str(), screenWidth - 100, 25, 20, LIGHTGRAY);
        DrawText(scorePrintable.c_str(), screenWidth / 2, 25, 20, LIGHTGRAY);
        EndDrawing();
        //----------------------------------------------------------------------------------
    }

    // De-Initialization
    //--------------------------------------------------------------------------------------
    CloseWindow();        // Close window and OpenGL context
    //--------------------------------------------------------------------------------------

    return 0;
}

Entity initBall(int screenWidth, int screenHeight) {
    Vector2 startPosition;
    startPosition.x = screenWidth / 2;
    startPosition.y = screenHeight / 2;
    Entity entity(startPosition, 10, 10);

    float randx = rand() * -2;
    float randy = rand();
    Vector2 startVelocity = {2, 2};
    entity.velocity = startVelocity;
    return entity; 
}

Player initPlayer(int screenWidth, int screenHeight) {
    Vector2 startPosition; 
    startPosition.x = screenWidth / 2.0;
    startPosition.y = (screenHeight - 50 ); 

    Player player(startPosition, 100, 20);
    player.init();
    return player;
}

void ballBounce(Entity* entity, char direction) {
    switch (direction) {
    case 't':
        entity->velocity.y *= -1; 
        break;
    
    case 'l':
        entity->velocity.x *= -1;
        break;

    case 'r':
        entity->velocity.x *= -1;
        break;
    default:
        entity->velocity.y = -1 * abs(entity->velocity.y);
    }
}

void createBricks(std::vector<Entity> &bricks) {
    int brickSizeX = 48;
    int brickSizeY = 10;

    int bricksPerRow = 20;
    int brickSpacing = 5; 
    int borderSpacing = 25;
    int sizeX = 1280 - (borderSpacing * 2);

    for (int i = 0; i < bricksPerRow; i++) {
        bricks.push_back({(50 + (50 * i)), 50, 48, 10});
    }
    for (int i = 0; i < bricksPerRow; i++) {
        bricks.push_back({(50 + (50 * i)), 65, 48, 10});
    }
    for (int i = 0; i < bricksPerRow; i++) {
        bricks.push_back({(50 + (50 * i)), 80, 48, 10});
    }
    for (int i = 0; i < bricksPerRow; i++) {
        bricks.push_back({(50 + (50 * i)), 95, 48, 10});
    }
}

void drawBricks(std::vector<Entity> &bricks) {
    for (int i = 0; i < bricks.size(); i++) {
        bricks[i].draw();
    }
}